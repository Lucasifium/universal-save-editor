//! Torchlight III (`<id>-<n>-CharacterData.sav` and the other `.sav` files).
//!
//! The file is an Unreal `GVAS` save whose only property, `JsonString`, is a string holding the
//! whole save as JSON text. The text is opened as JSON for editing; on saving it is put back as
//! the string and the two lengths in front of it are rewritten.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wrap {
    /// Everything before the text (up to and including the string's length).
    pub head: Vec<u8>,
    /// Everything after the text and its end byte.
    pub tail: Vec<u8>,
}

const NAME: &[u8] = b"\x0b\0\0\0JsonString\0\x0c\0\0\0StrProperty\0";

/// Opens a `GVAS` file with one `JsonString` property: (how to put the text back, the JSON text).
pub fn unpack(d: &[u8]) -> Option<(Wrap, Vec<u8>)> {
    if d.len() < 64 || &d[..4] != b"GVAS" {
        return None;
    }
    let at = d.windows(NAME.len()).position(|w| w == NAME)?;
    let size_at = at + NAME.len();
    let size = i64::from_le_bytes(d.get(size_at..size_at + 8)?.try_into().ok()?);
    let len_at = size_at + 9;
    let len = i32::from_le_bytes(d.get(len_at..len_at + 4)?.try_into().ok()?);
    if len < 3 || size != len as i64 + 4 {
        return None;
    }
    let start = len_at + 4;
    let end = start + len as usize - 1;
    if *d.get(end)? != 0 {
        return None;
    }
    let text = &d[start..end];
    let first = text.iter().find(|b| !b.is_ascii_whitespace())?;
    if (*first != b'{' && *first != b'[') || std::str::from_utf8(text).is_err() {
        return None;
    }
    Some((Wrap { head: d[..start].to_vec(), tail: d[end + 1..].to_vec() }, text.to_vec()))
}

pub fn pack(w: &Wrap, text: &[u8]) -> Result<Vec<u8>, String> {
    let len = text.len() as i32 + 1;
    let mut out = w.head.clone();
    let n = out.len();
    let size_at = n - 13;
    out[size_at..size_at + 8].copy_from_slice(&(len as i64 + 4).to_le_bytes());
    out[n - 4..].copy_from_slice(&len.to_le_bytes());
    out.extend_from_slice(text);
    out.push(0);
    out.extend_from_slice(&w.tail);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(text: &str) -> Vec<u8> {
        let mut d = b"GVAS\x02\0\0\0".to_vec();
        d.extend_from_slice(&[0; 40]);
        d.extend_from_slice(NAME);
        d.extend_from_slice(&(text.len() as i64 + 5).to_le_bytes());
        d.push(0);
        d.extend_from_slice(&(text.len() as i32 + 1).to_le_bytes());
        d.extend_from_slice(text.as_bytes());
        d.push(0);
        d.extend_from_slice(b"\x05\0\0\0None\0\0\0\0\0");
        d
    }

    #[test]
    fn text_round_trips_with_new_length() {
        let d = file("{\"a\": 1}");
        let (w, t) = unpack(&d).unwrap();
        assert_eq!(t, b"{\"a\": 1}");
        assert_eq!(pack(&w, &t).unwrap(), d);
        let longer = file("{\"a\": 12345}");
        assert_eq!(pack(&w, b"{\"a\": 12345}").unwrap(), longer);
    }
}
