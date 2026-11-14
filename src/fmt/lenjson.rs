//! A JSON document written by .NET's `BinaryWriter.Write(string)`: a 7-bit encoded length
//! (one to five bytes) followed by exactly that many bytes of UTF-8 text.

/// The JSON text when `data` is a length prefix followed by a JSON object of exactly that length.
pub fn open(data: &[u8]) -> Option<(Vec<u8>, usize)> {
    let (mut n, mut shift, mut i) = (0usize, 0u32, 0usize);
    loop {
        let b = *data.get(i)?;
        i += 1;
        n |= ((b & 0x7f) as usize).checked_shl(shift)?;
        if b < 0x80 {
            break;
        }
        shift += 7;
        if i >= 5 {
            return None;
        }
    }
    // The prefix is the file's first bytes; a plain text file starts with `{` (0x7b) or white space
    // and would otherwise be taken for a one-byte prefix.
    if i == 1 || n == 0 || data.len() != i + n || data.get(i) != Some(&b'{') {
        return None;
    }
    let body = &data[i..];
    serde_json::from_slice::<serde_json::Value>(body).ok()?;
    Some((body.to_vec(), i))
}

pub fn pack(json: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(json.len() + 5);
    let mut n = json.len();
    loop {
        let b = (n & 0x7f) as u8;
        n >>= 7;
        if n == 0 {
            out.push(b);
            break;
        }
        out.push(b | 0x80);
    }
    out.extend_from_slice(json);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let json = format!("{{\"a\": \"{}\"}}", "x".repeat(300)).into_bytes();
        let packed = pack(&json);
        assert_eq!(packed[0] & 0x80, 0x80);
        let (back, _) = open(&packed).unwrap();
        assert_eq!(back, json);
        assert_eq!(pack(&back), packed);
        assert!(open(&json).is_none());
    }
}
