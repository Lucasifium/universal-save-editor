//! Bloodstained: Ritual of the Night saves are an Unreal GVAS file XORed byte by byte with a
//! repeating counter: byte `i` is XORed with `(0x3A + i) mod 255`.
//!
//! The key stream is as published by Bloodstained_RotN_SaveConverter (DarkMarkZX, MIT) and
//! bloodsaved (jeromydean, MIT).

fn key(i: usize) -> u8 {
    ((0x3A + i) % 255) as u8
}

fn xor(d: &[u8]) -> Vec<u8> {
    d.iter().enumerate().map(|(i, b)| b ^ key(i)).collect()
}

/// The GVAS file inside and the bytes after its last property (a block that isn't part of it;
/// kept as it is).
pub fn open(data: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if data.len() < 32 || data.starts_with(b"GVAS") {
        return None;
    }
    let head: Vec<u8> = data[..4].iter().enumerate().map(|(i, b)| b ^ key(i)).collect();
    if head != b"GVAS" {
        return None;
    }
    let plain = xor(data);
    [20usize, 16, 4, 0].into_iter().find_map(|n| {
        let body = plain.get(..plain.len().checked_sub(n)?)?;
        super::gvas::parse(body).ok()?;
        Some((body.to_vec(), plain[plain.len() - n..].to_vec()))
    })
}

pub fn pack(plain: &[u8], tail: &[u8]) -> Vec<u8> {
    let mut all = plain.to_vec();
    all.extend_from_slice(tail);
    xor(&all)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xor_is_its_own_inverse() {
        let mut plain = b"GVAS".to_vec();
        plain.extend((0..600u32).map(|i| i as u8));
        assert_eq!(xor(&xor(&plain)), plain);
        assert_eq!(key(0), 0x3A);
        assert_eq!(key(197), 0);
    }
}
