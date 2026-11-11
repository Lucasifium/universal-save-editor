//! Ryu Ga Gotoku Studio "Dragon Engine" saves: the save data (JSON; MessagePack in Yakuza
//! Kiwami 3 and Pirate Yakuza; binary in Ishin), XORed with a repeating per-game key, then a
//! 32-bit check of the plain data. Keys, which games use which check and Ishin's 16-byte
//! trailer are from rgg_save_tool_py (MIT; keys found by its contributors), and the
//! Adler-32-from-0 check from SutandoTsukai181's yk2_save. Kiwami 2 and Like a Dragon were
//! checked against sample saves.

/// (games, key)
pub const KEYS: [(&str, &[u8]); 6] = [
    ("Kiwami 2, Like a Dragon, Infinite Wealth, Kiwami 3, Pirate Yakuza", b"STarYZgr3DL11"),
    ("Like a Dragon (GOG)", b"r3DL11STarYZg"),
    ("Yakuza 6", b"VI3rbPckNsea7JOUMrgT"),
    ("Judgment", b"OphYnzbPoV5lj"),
    ("Lost Judgment, Gaiden", b"jKMQEv7S4l9hd"),
    ("Ishin", b"fuEw5rWN8MBS"),
];
const ISHIN: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check {
    Crc32,
    Adler0,
}

/// How a save is wrapped: which key, which check, and (Ishin) the 16 bytes after the data,
/// which hold the check at 8..12.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wrap {
    pub key: u8,
    pub check: Check,
    pub trailer: Option<[u8; 16]>,
}

fn xor(d: &[u8], key: &[u8]) -> Vec<u8> {
    d.iter().enumerate().map(|(i, b)| b ^ key[i % key.len()]).collect()
}

fn sum(check: Check, d: &[u8]) -> u32 {
    match check {
        Check::Crc32 => {
            let mut h = flate2::Crc::new();
            h.update(d);
            h.sum()
        }
        Check::Adler0 => {
            let (mut a, mut b) = (0u32, 0u32);
            for chunk in d.chunks(5552) {
                for &x in chunk {
                    a += u32::from(x);
                    b += a;
                }
                a %= 65521;
                b %= 65521;
            }
            (b << 16) | a
        }
    }
}

/// Does `first` (the first plain bytes) look like the start of this kind of save?
fn plausible(key: usize, first: &[u8]) -> bool {
    match key {
        // Ishin's binary data starts with a small u32.
        ISHIN => first.len() >= 4 && first[1..4] == [0, 0, 0],
        // JSON, or a MessagePack map.
        _ => first.starts_with(b"{\"") || matches!(first.first(), Some(0x80..=0x8F | 0xDE | 0xDF)),
    }
}

/// The plain save and how it was wrapped.
pub fn decrypt(d: &[u8]) -> Option<(Wrap, Vec<u8>)> {
    if d.len() < 32 {
        return None;
    }
    for (k, (_, key)) in KEYS.iter().enumerate() {
        let first = xor(&d[..4], key);
        if !plausible(k, &first) {
            continue;
        }
        let tail = if k == ISHIN { 16 } else { 4 };
        let body = &d[..d.len() - tail];
        let stored = if k == ISHIN { &d[d.len() - 8..d.len() - 4] } else { &d[d.len() - 4..] };
        let stored = u32::from_le_bytes(stored.try_into().unwrap());
        let plain = xor(body, key);
        for check in [Check::Crc32, Check::Adler0] {
            if sum(check, &plain) == stored {
                let trailer = (k == ISHIN).then(|| d[d.len() - 16..].try_into().unwrap());
                return Some((Wrap { key: k as u8, check, trailer }, plain));
            }
        }
    }
    None
}

pub fn encrypt(w: &Wrap, plain: &[u8]) -> Vec<u8> {
    let key = KEYS[w.key as usize].1;
    let mut out = xor(plain, key);
    let s = sum(w.check, plain).to_le_bytes();
    match w.trailer {
        Some(mut t) => {
            t[8..12].copy_from_slice(&s);
            out.extend(t);
        }
        None => out.extend(s),
    }
    out
}

/// Which games a save's key belongs to (for the format's name).
pub fn games(w: &Wrap) -> &'static str {
    KEYS[w.key as usize].0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_round_trips() {
        assert_eq!(sum(Check::Adler0, b"Wikipedia"), 0x11DD_0397);
        let json = br#"{".ver":1,"player":{"point":{"m_point":[0,1500]}}}"#.to_vec();
        let ishin = [20u8, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8].repeat(8);
        for k in 0..KEYS.len() {
            let plain = if k == ISHIN { ishin.clone() } else { json.clone() };
            for check in [Check::Crc32, Check::Adler0] {
                let w = Wrap { key: k as u8, check, trailer: (k == ISHIN).then_some([7u8; 16]) };
                let file = encrypt(&w, &plain);
                let (w2, back) = decrypt(&file).unwrap_or_else(|| panic!("key {k}"));
                assert_eq!(back, plain);
                assert_eq!(encrypt(&w2, &back), file);
            }
        }
    }

    /// Research aid: SAVE_EDITOR_RGG = a data.sav; checks decode → encode gives the file back.
    #[test]
    #[ignore]
    fn real_file() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_RGG").unwrap()).unwrap();
        let (w, plain) = decrypt(&d).expect("decrypt");
        assert_eq!(encrypt(&w, &plain), d);
        println!("{w:?} ({}); {} bytes", games(&w), plain.len());
    }
}
