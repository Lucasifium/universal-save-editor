//! LEGO Batman: Legacy of the Dark Knight (TT Games, Unreal Engine 5): the whole save file
//! is RC4-encrypted with one 32-byte key built into the game, and what is under it is a
//! normal Unreal `GVAS` save that `gvas.rs` reads. RC4 is its own inverse, so one function
//! opens and closes the file. The key is the one published by the game's save decryptor
//! (RealDarkCraft, 2026).
//!
//! Studs are kept in several places (`StudsCollected` and two `Saved_Total` fields); the
//! game screen writes them all together.

const KEY: [u8; 32] = [
    0x21, 0x38, 0x11, 0x60, 0x17, 0x47, 0x2f, 0x53, 0x5d, 0x37, 0x24, 0x0e, 0x0e, 0x0f, 0x60, 0x43, 0x2f, 0x0e, 0x3f, 0x0a, 0x27, 0x55, 0x4b, 0x0b, 0x4f, 0x59, 0x25, 0x38, 0x0b, 0x3a, 0x44,
    0x17,
];

pub const KIND: &str = "LEGO Batman: Legacy of the Dark Knight save";

pub fn rc4(data: &[u8]) -> Vec<u8> {
    let mut s: Vec<u8> = (0..=255).collect();
    let mut j = 0usize;
    for i in 0..256 {
        j = (j + s[i] as usize + KEY[i % KEY.len()] as usize) & 255;
        s.swap(i, j);
    }
    let (mut i, mut j) = (0usize, 0usize);
    data.iter()
        .map(|&b| {
            i = (i + 1) & 255;
            j = (j + s[i] as usize) & 255;
            s.swap(i, j);
            b ^ s[(s[i] as usize + s[j] as usize) & 255]
        })
        .collect()
}

/// The decrypted GVAS bytes when `d` is an encrypted save of this game.
pub fn open(d: &[u8]) -> Option<Vec<u8>> {
    if d.len() < 64 || d.starts_with(b"GVAS") || rc4(&d[..4]) != b"GVAS" {
        return None;
    }
    // Four bytes spelling "GVAS" after decrypting is enough to tell it from any other file.
    Some(rc4(d))
}

/// Every pointer to a property whose name (without the ` (n)` suffix the tree adds to repeats)
/// is `name`.
pub fn find_all(root: &serde_json::Value, name: &str) -> Vec<String> {
    fn walk(v: &serde_json::Value, path: &str, name: &str, out: &mut Vec<String>) {
        match v {
            serde_json::Value::Object(m) => {
                for (k, c) in m {
                    let p = format!("{path}/{}", crate::tree::esc(k));
                    let bare = k.rsplit_once(" (").filter(|(_, n)| n.ends_with(')')).map_or(k.as_str(), |(a, _)| a);
                    if bare == name && !c.is_object() && !c.is_array() {
                        out.push(p.clone());
                    }
                    walk(c, &p, name, out);
                }
            }
            serde_json::Value::Array(a) => {
                for (i, c) in a.iter().enumerate() {
                    walk(c, &format!("{path}/{i}"), name, out);
                }
            }
            _ => {}
        }
    }
    let mut out = vec![];
    walk(root, "", name, &mut out);
    out
}

/// A GVAS save that keeps the studs wallet is this game's.
pub fn is_lego(root: &serde_json::Value) -> bool {
    !find_all(root, "StudsCollected").is_empty()
}

pub fn close(data: &[u8]) -> Vec<u8> {
    rc4(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rc4_round_trip_and_detection() {
        let mut gvas = b"GVAS".to_vec();
        gvas.extend((0..200u8).map(|x| x.wrapping_mul(7)));
        let enc = close(&gvas);
        assert_ne!(&enc[..4], b"GVAS");
        assert_eq!(open(&enc).unwrap(), gvas);
        assert!(open(&gvas).is_none());
    }
}
