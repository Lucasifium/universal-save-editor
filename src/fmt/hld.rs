//! Hyper Light Drifter save: a hex header, then base64 of a JSON text ending in a NUL byte.
//! Facts about the layout come from the MIT-licensed hld-save-transfer tool; the code here is new.

use base64::{engine::general_purpose::STANDARD, Engine};

const MARK: &str = "eyAi"; // base64 of `{ "`

/// Returns the header text and the JSON bytes.
pub fn open(data: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    let text = std::str::from_utf8(data).ok()?;
    let at = text.find(MARK)?;
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

pub fn pack(head: &[u8], json: &[u8]) -> Vec<u8> {
    let mut body = json.to_vec();
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
        let file = pack(b"0123456789abcdef", br#"{ "sword": 1.000000 }"#);
        let (head, json) = open(&file).unwrap();
        assert_eq!(head, b"0123456789abcdef");
        assert_eq!(pack(&head, &json), file);
    }
}
