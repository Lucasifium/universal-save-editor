//! Hyper Light Drifter save: a hex header, then base64 of a JSON text ending in a NUL byte.
//! Facts about the layout come from the MIT-licensed hld-save-transfer tool; the code here is new.

use base64::{engine::general_purpose::STANDARD, Engine};


/// Returns the header text and the JSON bytes.
pub fn open(data: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    // The header is all hex digits up to the mark: data that has a byte other than a hex digit
    // in its first 16 is turned down before the whole text is checked and searched.
    if data.iter().take(16).any(|b| !b.is_ascii_hexdigit()) || data.len() < 16 {
        return None;
    }
    let text = std::str::from_utf8(data).ok()?;
    // The base64 starts with `ey` (`{` and then a space or a quote). `e` is a hex digit, so the
    // header ends one character before the first character that is not one.
    let at = text.bytes().position(|b| !b.is_ascii_hexdigit())?.checked_sub(1)?;
    if !text[at..].starts_with("ey") {
        return None;
    }
    let (head, body) = text.split_at(at);
    if head.len() < 16 || !head.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let body = body.trim_end_matches(['\r', '\n']);
    let mut json = STANDARD.decode(body).ok()?;
    if json.pop() != Some(0) || !json.starts_with(b"{") {
        return None;
    }
    Some((head.as_bytes().to_vec(), json))
}

/// `original` is the JSON text the file held: when the edited JSON has the same data, the text
/// is kept as it was (spacing and number formats), not written out again.
pub fn pack(head: &[u8], original: &[u8], json: &[u8]) -> Vec<u8> {
    let same = |a: &[u8], b: &[u8]| matches!((serde_json::from_slice::<serde_json::Value>(a), serde_json::from_slice::<serde_json::Value>(b)), (Ok(x), Ok(y)) if x == y);
    let mut body = if same(original, json) { original.to_vec() } else { json.to_vec() };
    body.push(0);
    let mut out = head.to_vec();
    out.extend_from_slice(STANDARD.encode(&body).as_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let file = pack(b"0123456789abcdef", b"", br#"{ "sword": 1.000000 }"#);
        let (head, json) = open(&file).unwrap();
        assert_eq!(head, b"0123456789abcdef");
        assert_eq!(pack(&head, &json, &json), file);
        // Edited data written compactly still opens.
        let edited = pack(&head, &json, br#"{"sword":2.0}"#);
        assert_eq!(open(&edited).unwrap().1, br#"{"sword":2.0}"#);
    }
}
