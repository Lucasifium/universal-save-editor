//! Ryu Ga Gotoku Studio "Dragon Engine" saves (Yakuza Kiwami 2, Like a Dragon and later):
//! the save is JSON, XORed with a repeating 13-byte key, then a 32-bit check of the plain
//! JSON, stored as is. Kiwami 2 uses Adler-32 started at 0 instead of 1 (facts from
//! SutandoTsukai181's yk2_save; the key is also in simontime's notes); Like a Dragon uses
//! CRC-32. Both were checked against sample saves.

const KEY: &[u8; 13] = b"STarYZgr3DL11";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check {
    Crc32,
    Adler0,
}

fn xor(d: &[u8]) -> Vec<u8> {
    d.iter().enumerate().map(|(i, b)| b ^ KEY[i % KEY.len()]).collect()
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

/// The plain JSON and which check the file uses.
pub fn decrypt(d: &[u8]) -> Option<(Check, Vec<u8>)> {
    if d.len() < 16 || d[0] ^ KEY[0] != b'{' || d[1] ^ KEY[1] != b'"' {
        return None;
    }
    let plain = xor(&d[..d.len() - 4]);
    let stored = u32::from_le_bytes(d[d.len() - 4..].try_into().unwrap());
    [Check::Crc32, Check::Adler0].into_iter().find(|&c| sum(c, &plain) == stored).map(|c| (c, plain))
}

pub fn encrypt(check: Check, plain: &[u8]) -> Vec<u8> {
    let mut out = xor(plain);
    out.extend(sum(check, plain).to_le_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_adler() {
        // Adler-32 from 0: "Wikipedia" is 0x11E60398 with the usual start of 1.
        assert_eq!(sum(Check::Adler0, b"Wikipedia"), 0x11DD_0397);
        for c in [Check::Crc32, Check::Adler0] {
            let plain = br#"{".ver":1,"player":{"point":{"m_point":[0,1500]}}}"#;
            let file = encrypt(c, plain);
            assert_eq!(decrypt(&file), Some((c, plain.to_vec())));
        }
    }

    /// Research aid: SAVE_EDITOR_RGG = a data.sav; checks decode → encode gives the file back.
    #[test]
    #[ignore]
    fn real_file() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_RGG").unwrap()).unwrap();
        let (c, plain) = decrypt(&d).expect("decrypt");
        assert_eq!(encrypt(c, &plain), d);
        let v: serde_json::Value = serde_json::from_slice(&plain).unwrap();
        let again = serde_json::to_vec(&v).unwrap();
        println!("{c:?}; JSON {} bytes; re-serialised identically: {}", plain.len(), again == plain);
    }
}
