//! Persona 5 Royal (PC) `DATA.DAT` / `SYSTEM.DAT`. Container facts from fiber-saveutil
//! (ciferkey): "DATA", a CRC-32 (polynomial 0x04C11DB7, start 0xFFFFFFFF, not reflected,
//! no final XOR) of everything after it, a timestamp, flags (the top bit = encrypted) and
//! a 16-byte IV. From 0x20 the rest is AES-256-CBC: the sizes (header, packed header,
//! data, packed data), the save's flags (1 = header zlib, 2 = data zlib) and a CRC of the
//! unpacked data, then the load-screen header and the game data, each padded to 16 bytes.

use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};

const KEY: [u8; 32] = [
    0xde, 0x53, 0x99, 0x4b, 0x49, 0x18, 0x4a, 0x83, 0x8e, 0xb6, 0x40, 0xb8, 0x73, 0xb2, 0x03, 0x7e,
    0xf3, 0x57, 0x9f, 0x12, 0x29, 0xad, 0x43, 0xd3, 0x95, 0x41, 0x95, 0x0b, 0x7c, 0x81, 0x24, 0x5d,
];

pub fn crc(d: &[u8]) -> u32 {
    static TABLE: std::sync::OnceLock<[u32; 256]> = std::sync::OnceLock::new();
    let t = TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        for (i, e) in t.iter_mut().enumerate() {
            let mut c = (i as u32) << 24;
            for _ in 0..8 {
                c = if c & 0x8000_0000 != 0 {
                    (c << 1) ^ 0x04C1_1DB7
                } else {
                    c << 1
                };
            }
            *e = c;
        }
        t
    });
    d.iter().fold(0xFFFF_FFFF, |c, &b| {
        (c << 8) ^ t[((c >> 24) as u8 ^ b) as usize]
    })
}

fn u32_at(d: &[u8], o: usize) -> usize {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap()) as usize
}
fn u16_at(d: &[u8], o: usize) -> usize {
    u16::from_le_bytes(d[o..o + 2].try_into().unwrap()) as usize
}
fn align(v: usize) -> usize {
    (v + 15) & !15
}

fn iv(d: &[u8]) -> [u8; 16] {
    d[0x10..0x20].try_into().unwrap()
}

/// The file with its encrypted part decrypted (the flags are left as they are).
fn plain(d: &[u8]) -> Option<Vec<u8>> {
    if d.len() < 0x40 || &d[..4] != b"DATA" || (d.len() - 0x20) % 16 != 0 {
        return None;
    }
    if crc(&d[8..]) != u32_at(d, 4) as u32 {
        return None;
    }
    let mut out = d.to_vec();
    if u32_at(d, 12) >> 31 == 1 {
        let mut c = cbc::Decryptor::<aes::Aes256>::new(&KEY.into(), &iv(d).into());
        for b in out[0x20..].chunks_exact_mut(16) {
            c.decrypt_block_mut(b.into());
        }
    }
    Some(out)
}

/// What stays the same when the game data changes (the file up to the data), and the
/// unpacked game data.
pub fn unpack(d: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    let p = plain(d)?;
    let (hs, hsc, ds, dsc, flags) = (
        u16_at(&p, 0x20),
        u16_at(&p, 0x22),
        u32_at(&p, 0x24),
        u32_at(&p, 0x28),
        u32_at(&p, 0x2C),
    );
    let at = align(if flags & 1 != 0 { hsc } else { hs });
    if at < 0x40 || at > p.len() {
        return None;
    }
    let data = if flags & 2 != 0 {
        let raw = p.get(at..at + dsc)?;
        let mut out = Vec::with_capacity(ds);
        std::io::Read::read_to_end(&mut flate2::read::ZlibDecoder::new(raw), &mut out).ok()?;
        out
    } else {
        p.get(at..at + ds)?.to_vec()
    };
    if data.len() != ds || crc(&data) != u32_at(&p, 0x30) as u32 {
        return None;
    }
    Some((p[..at].to_vec(), data))
}

pub fn pack(head: &[u8], data: &[u8]) -> Vec<u8> {
    let mut out = head.to_vec();
    let flags = u32_at(head, 0x2C);
    out[0x24..0x28].copy_from_slice(&(data.len() as u32).to_le_bytes());
    out[0x30..0x34].copy_from_slice(&crc(data).to_le_bytes());
    let body = if flags & 2 != 0 {
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
        std::io::Write::write_all(&mut z, data).unwrap();
        let z = z.finish().unwrap();
        out[0x28..0x2C].copy_from_slice(&(z.len() as u32).to_le_bytes());
        z
    } else {
        data.to_vec()
    };
    out.extend(&body);
    out.resize(align(out.len()), 0);
    if u32_at(head, 12) >> 31 == 1 {
        let mut c = cbc::Encryptor::<aes::Aes256>::new(&KEY.into(), &iv(head).into());
        for b in out[0x20..].chunks_exact_mut(16) {
            c.encrypt_block_mut(b.into());
        }
    }
    let sum = crc(&out[8..]);
    out[4..8].copy_from_slice(&sum.to_le_bytes());
    out
}

// ------------------------------------------------------------------ the game data
//
// Worked out from sample PC saves (data version 0x31): ten party records of 0x2B0 bytes
// from 0x18 — the character's number at +0x10, HP +0x14, SP +0x18, the protagonist's own
// level +0x20 and experience +0x24, then 12 Persona slots of 0x30 bytes from +0x4C (the
// Persona layout as documented for the PS4 saves: id +2, level +4, experience +8, eight
// skills, then strength, magic, endurance, agility and luck from +0x1C). The party's
// levels are their Persona's. A second copy of the table sits at 0x18528. Money is the
// u32 at 0x35C0.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const DATA_LEN: usize = 0x30720;
const TABLE: usize = 0x18;
const COPY: usize = 0x18528;
const REC: usize = 0x2B0;
const MONEY: usize = 0x35C0;
const NAMES: [&str; 10] = [
    "Joker", "Ryuji", "Morgana", "Ann", "Yusuke", "Makoto", "Haru", "Futaba", "Akechi", "Kasumi",
];

fn id(d: &[u8], c: usize) -> usize {
    u32_at(d, TABLE + REC * c + 0x10)
}

pub fn looks(d: &[u8]) -> bool {
    d.len() == DATA_LEN && (0..10).all(|c| id(d, c) == c + 1)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not Persona 5 Royal game data".into());
    }
    let mut b = Builder::new(d);
    for (c, who) in NAMES.into_iter().enumerate() {
        let r = TABLE + REC * c;
        let twin = COPY + REC * c;
        let same = |o: usize, n: usize| d.get(twin + o..twin + o + n) == d.get(r + o..r + o + n);
        let mut f = |k: &str, o: usize, kind: Kind, lo: i64, hi: i64, n: usize| {
            b.ranged(&[who, k], r + o, kind, lo, hi);
            if same(o, n) {
                b.mirror(&[who, k], twin + o);
            }
        };
        f("HP", 0x14, Kind::I32, 0, 999, 4);
        f("SP", 0x18, Kind::I32, 0, 999, 4);
        if c == 0 {
            f("Level", 0x20, Kind::I32, 1, 99, 4);
            f("Experience", 0x24, Kind::I32, 0, 99_999_999, 4);
        }
        for slot in 0..if c == 0 { 12 } else { 1 } {
            let p = 0x4C + 0x30 * slot;
            let pid = u16::from_le_bytes([d[r + p + 2], d[r + p + 3]]);
            if pid == 0 {
                continue;
            }
            let label = if c == 0 {
                format!("Persona {:02} (#{pid})", slot + 1)
            } else {
                "Persona".to_string()
            };
            let mut g = |k: &str, o: usize, kind: Kind, lo: i64, hi: i64, n: usize| {
                b.ranged(&[who, &label, k], r + p + o, kind, lo, hi);
                if same(p + o, n) {
                    b.mirror(&[who, &label, k], twin + p + o);
                }
            };
            g("Level", 4, Kind::U8, 1, 99, 1);
            g("Experience", 8, Kind::I32, 0, 99_999_999, 4);
            for (i, s) in ["Strength", "Magic", "Endurance", "Agility", "Luck"]
                .into_iter()
                .enumerate()
            {
                g(s, 0x1C + i, Kind::U8, 1, 99, 1);
            }
        }
    }
    // Money: after the item counts (found by comparing saves from different days).
    if u32_at(d, MONEY) <= 9_999_999 {
        b.ranged(&["Joker", "Money"], MONEY, Kind::U32, 0, 9_999_999);
    }
    Ok(b.finish("Persona 5 Royal save", Checksum::None).with_note(
        "Money, and HP, SP, levels, experience and Persona stats for the Phantom Thieves (a party member's level is \
         their Persona's).",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_check_value() {
        // CRC-32/MPEG-2 check value.
        assert_eq!(crc(b"123456789"), 0x0376_E6E7);
    }

    #[test]
    fn round_trip() {
        let mut head = vec![0u8; 0x40];
        head[..4].copy_from_slice(b"DATA");
        head[12..16].copy_from_slice(&0x8000_0000u32.to_le_bytes());
        head[0x10..0x20].copy_from_slice(&[9; 16]);
        head[0x20..0x22].copy_from_slice(&0x40u16.to_le_bytes());
        head[0x2C..0x30].copy_from_slice(&2u32.to_le_bytes());
        let data: Vec<u8> = (0..5000u32).map(|i| (i % 7) as u8).collect();
        let file = pack(&head, &data);
        let (h, d) = unpack(&file).unwrap();
        assert_eq!(d, data);
        assert_eq!(pack(&h, &d), file);
    }

    /// Research aid: SAVE_EDITOR_P5R = a DATA.DAT; writes the data next to it as .plain.
    #[test]
    #[ignore]
    fn real_file() {
        let path = std::env::var("SAVE_EDITOR_P5R").unwrap();
        let d = std::fs::read(&path).unwrap();
        let (h, data) = unpack(&d).expect("unpack");
        println!(
            "head {} data {} flags {:x}",
            h.len(),
            data.len(),
            u32_at(&h, 0x2C)
        );
        let again = pack(&h, &data);
        assert_eq!(unpack(&again).unwrap().1, data);
        std::fs::write(format!("{path}.plain"), &data).unwrap();
        std::fs::write(format!("{path}.head"), &h).unwrap();
    }
}
