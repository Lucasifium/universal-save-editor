//! Moving a save to another Steam account: finds the 64-bit Steam IDs stored inside the save
//! (as 8 little-endian bytes in binary data, or as numbers or text in JSON and the Tree) so
//! they can all be changed at once. The save's own packing, encryption and checksums are
//! redone when it is saved, as for any edit.

use serde_json::Value;

/// The first individual-account Steam ID; account number n is `BASE + n`.
pub const BASE: u64 = 76_561_197_960_265_728;

pub fn is_id(v: u64) -> bool {
    (BASE + 1..BASE + (1 << 32)).contains(&v)
}

/// Turns what the player typed (a 17-digit Steam ID or an account number) into a Steam ID.
pub fn parse(text: &str) -> Option<u64> {
    let n: u64 = text.trim().parse().ok()?;
    let id = if n < 1 << 32 { BASE + n } else { n };
    is_id(id).then_some(id)
}

/// Each Steam ID in binary data and where it is (byte offsets).
pub fn in_bytes(d: &[u8]) -> Vec<(u64, Vec<usize>)> {
    let mut out: Vec<(u64, Vec<usize>)> = vec![];
    // Every ID ends in the bytes 01 00 10 01 (little-endian), so look for those first.
    for i in 4..d.len().saturating_sub(3) {
        if d[i..i + 4] != [0x01, 0x00, 0x10, 0x01] {
            continue;
        }
        let v = u64::from_le_bytes(d[i - 4..i + 4].try_into().unwrap());
        if is_id(v) {
            match out.iter_mut().find(|(id, _)| *id == v) {
                Some((_, at)) => at.push(i - 4),
                None => out.push((v, vec![i - 4])),
            }
        }
    }
    out
}

fn walk(v: &Value, ptr: String, out: &mut Vec<(u64, Vec<String>)>) {
    match v {
        Value::Object(m) => m.iter().for_each(|(k, c)| walk(c, crate::tree::child(&ptr, k), out)),
        Value::Array(a) => a.iter().enumerate().for_each(|(i, c)| walk(c, format!("{ptr}/{i}"), out)),
        Value::Number(n) => {
            if let Some(id) = n.as_u64().filter(|&x| is_id(x)) {
                add(out, id, ptr);
            }
        }
        Value::String(s) => {
            if let Some(id) = s.parse::<u64>().ok().filter(|&x| is_id(x) && s.len() == 17) {
                add(out, id, ptr);
            }
        }
        _ => {}
    }
}

fn add(out: &mut Vec<(u64, Vec<String>)>, id: u64, ptr: String) {
    match out.iter_mut().find(|(x, _)| *x == id) {
        Some((_, p)) => p.push(ptr),
        None => out.push((id, vec![ptr])),
    }
}

/// Each Steam ID among the values and the pointers to it.
pub fn in_values(v: &Value) -> Vec<(u64, Vec<String>)> {
    let mut out = vec![];
    walk(v, String::new(), &mut out);
    out
}

/// The same kind of value (number or text) holding `new`.
pub fn replaced(old: &Value, new: u64) -> Value {
    match old {
        Value::String(_) => Value::String(new.to_string()),
        _ => Value::from(new),
    }
}

/// The Steam accounts that have played on this computer.
#[cfg(not(target_arch = "wasm32"))]
pub fn local_accounts() -> Vec<u64> {
    let mut out = vec![];
    for dir in crate::platform::steam_userdata_dirs() {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            if let Some(id) = e.file_name().to_str().and_then(|n| n.parse::<u64>().ok()).filter(|&n| n > 0 && n < 1 << 32) {
                if !out.contains(&(BASE + id)) {
                    out.push(BASE + id);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_ids() {
        let id = BASE + 123_456;
        let mut d = vec![0u8; 40];
        d[3..11].copy_from_slice(&id.to_le_bytes());
        d[30..38].copy_from_slice(&id.to_le_bytes());
        assert_eq!(in_bytes(&d), vec![(id, vec![3, 30])]);
        let v = serde_json::json!({"owner": id, "list": [{"sid": id.to_string()}], "n": 5});
        assert_eq!(in_values(&v), vec![(id, vec!["/owner".to_string(), "/list/0/sid".to_string()])]);
        assert_eq!(parse("123456"), Some(id));
        assert_eq!(parse(&id.to_string()), Some(id));
        assert_eq!(parse("5"), Some(BASE + 5));
        assert_eq!(parse("abc"), None);
        assert_eq!(replaced(&Value::String(String::new()), id), Value::String(id.to_string()));
    }
}
