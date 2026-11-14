//! Mad Max saves: every byte XORed with a fixed 32-byte key, a 0x33116F7F magic after an
//! integrity value, and the payload usually written twice. Only the scrap amount is exposed.
//! The integrity value can't be computed from scratch; like the public layout notes say, the
//! difference between the stored value and a CRC-32 variant depends only on the file's length, so
//! it is carried over from the opened file (edits never change the length). Facts only; the code
//! is new.

use serde_json::{json, Value};

const KEY: [u8; 32] = [
    0xea, 0xd5, 0xba, 0xd5, 0xee, 0xd5, 0xab, 0xee, 0xba, 0x57, 0xab, 0xaa, 0xab, 0xba, 0xab, 0xee, 0xba, 0x75, 0xaa, 0x57, 0xae, 0xd5, 0xab, 0xea, 0xba,
    0x75, 0xae, 0x75, 0xab, 0xab, 0x5d, 0xd5,
];
const MAGIC: u32 = 0x3311_6F7F;

fn xor(d: &[u8]) -> Vec<u8> {
    d.iter().enumerate().map(|(i, b)| b ^ KEY[i % 32]).collect()
}

fn crc(d: &[u8]) -> u32 {
    let mut r = 0xBCEA_C598u32;
    for &b in d {
        r ^= b as u32;
        for _ in 0..8 {
            r = if r & 1 != 0 { (r >> 1) ^ 0xEDB8_8320 } else { r >> 1 };
        }
    }
    r ^ 0x7B7A_D18A
}

fn u32_at(d: &[u8], p: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(p..p + 4)?.try_into().ok()?))
}

/// Offsets of the scrap value: the record that follows `k, 4, k+1` / `k+1, 4, m` / `k+2, 4, scrap` / `k+3, 1, 1`.
fn scrap_spots(d: &[u8]) -> Vec<usize> {
    let mut out = vec![];
    if d.len() < 48 {
        return out;
    }
    for i in 0..d.len() - 41 {
        let Some(k) = u32_at(d, i) else { break };
        if u32_at(d, i + 4) == Some(4)
            && u32_at(d, i + 8) == Some(k.wrapping_add(1))
            && u32_at(d, i + 12) == Some(k.wrapping_add(1))
            && u32_at(d, i + 16) == Some(4)
            && u32_at(d, i + 24) == Some(k.wrapping_add(2))
            && u32_at(d, i + 28) == Some(4)
            && u32_at(d, i + 36) == Some(k.wrapping_add(3))
            && u32_at(d, i + 40) == Some(1)
            && d.get(i + 44) == Some(&1)
        {
            out.push(i + 32);
        }
    }
    out
}

/// The decoded file and the scrap amount as JSON.
pub fn open(data: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if data.len() < 0x100 || data.len() % 512 != 0 {
        return None;
    }
    let plain = xor(data);
    if u32_at(&plain, 4)? != MAGIC {
        return None;
    }
    let spots = scrap_spots(&plain);
    let at = *spots.first()?;
    let scrap = f32::from_le_bytes(plain.get(at..at + 4)?.try_into().ok()?);
    // Whole amounts are shown as integers so that typing a whole number reads back the same.
    let shown = if scrap.fract() == 0.0 && scrap.abs() < 1e9 { json!(scrap as i64) } else { json!(scrap) };
    let json = serde_json::to_vec_pretty(&json!({ "Mad Max scrap": shown })).ok()?;
    Some((plain, json))
}

pub fn pack(original: &[u8], json: &[u8]) -> Result<Vec<u8>, String> {
    let v: Value = serde_json::from_slice(json).map_err(|e| e.to_string())?;
    let scrap = v.get("Mad Max scrap").and_then(Value::as_f64).filter(|n| n.is_finite() && *n >= 0.0).ok_or("Mad Max scrap must be a number")? as f32;
    let spots = scrap_spots(original);
    if spots.is_empty() {
        return Err("The Mad Max save this was opened from is not valid".into());
    }
    let delta = u32_at(original, 0).ok_or("short file")? ^ crc(&original[4..]);
    let mut out = original.to_vec();
    for at in spots {
        out[at..at + 4].copy_from_slice(&scrap.to_le_bytes());
    }
    let sum = crc(&out[4..]) ^ delta;
    out[0..4].copy_from_slice(&sum.to_le_bytes());
    Ok(xor(&out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut d = vec![0u8; 1024];
        d[4..8].copy_from_slice(&MAGIC.to_le_bytes());
        let rec: Vec<u8> = [46u32, 4, 47, 47, 4, 0x3f80_0000, 48, 4, 1000.0f32.to_bits(), 49, 1]
            .iter()
            .flat_map(|n| n.to_le_bytes())
            .chain([1u8])
            .collect();
        d[0x100..0x100 + rec.len()].copy_from_slice(&rec);
        d[0x200..0x200 + rec.len()].copy_from_slice(&rec);
        let c = crc(&d[4..]) ^ 0x1234_5678;
        d[0..4].copy_from_slice(&c.to_le_bytes());
        xor(&d)
    }

    #[test]
    fn edits_scrap_in_both_copies_and_keeps_the_check_delta() {
        let file = sample();
        let (plain, json) = open(&file).unwrap();
        let mut v: Value = serde_json::from_slice(&json).unwrap();
        assert_eq!(v["Mad Max scrap"], 1000);
        v["Mad Max scrap"] = json!(5000);
        let out = pack(&plain, &serde_json::to_vec(&v).unwrap()).unwrap();
        assert_eq!(out.len(), file.len());
        let (p2, j2) = open(&out).unwrap();
        assert_eq!(serde_json::from_slice::<Value>(&j2).unwrap()["Mad Max scrap"], 5000);
        assert_eq!(scrap_spots(&p2).len(), 2);
        assert_eq!(u32_at(&p2, 0).unwrap() ^ crc(&p2[4..]), 0x1234_5678);
    }
}
