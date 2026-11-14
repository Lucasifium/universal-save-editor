//! Cookie Clicker save text: base64 (often URL-escaped, ending in `!END!`) of `version||times|prefs|…` with
//! `|` between sections and `;` between values. A few values are shown as JSON and patched into the text;
//! every other character of the save is kept.

use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Map, Value};

const BUILDINGS: [&str; 20] = [
    "Cursor", "Grandma", "Farm", "Mine", "Factory", "Bank", "Temple", "Wizard tower", "Shipment", "Alchemy lab",
    "Portal", "Time machine", "Antimatter condenser", "Prism", "Chancemaker", "Fractal engine", "Javascript console",
    "Idleverse", "Cortex baker", "You",
];

/// Section 4, `;` separated: (JSON key, index).
const STATS: [(&str, usize); 9] = [
    ("cookies", 0),
    ("cookiesEarned", 1),
    ("cookieClicks", 2),
    ("goldenClicks", 3),
    ("handmadeCookies", 4),
    ("cookiesBeforeAscending", 8),
    ("ascensions", 14),
    ("wrinklersPopped", 17),
    ("santaLevel", 18),
];

/// How the text was wrapped, to wrap it the same way again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wrap {
    pub escaped_padding: bool,
    pub end: String,
}

fn num(tok: &str) -> Value {
    match tok.trim().parse::<f64>() {
        Ok(f) if f.is_finite() => {
            if f.fract() == 0.0 && f.abs() < 9e15 {
                json!(f as i64)
            } else {
                json!(f)
            }
        }
        _ => Value::Null,
    }
}

fn js_number(f: f64) -> String {
    if f.fract() == 0.0 && f.abs() < 1e21 {
        return format!("{}", f as i128);
    }
    let s = format!("{:e}", f);
    match s.split_once('e') {
        Some((m, e)) if !e.starts_with('-') => format!("{m}e+{e}"),
        _ => s,
    }
}

/// Replaces `tok` by the new value when it really changed.
fn patch(tok: &str, new: Option<f64>) -> String {
    match (new, tok.trim().parse::<f64>()) {
        (Some(n), Ok(old)) if n != old => js_number(n),
        (Some(n), Err(_)) if n.is_finite() => js_number(n),
        _ => tok.to_string(),
    }
}

pub fn open(data: &[u8]) -> Option<(Wrap, Vec<u8>, Vec<u8>)> {
    let text = std::str::from_utf8(data).ok()?.trim();
    if text.len() < 40 || text.contains(char::is_whitespace) {
        return None;
    }
    let (body, end) = if let Some(b) = text.strip_suffix("%21END%21") {
        (b, "%21END%21")
    } else if let Some(b) = text.strip_suffix("!END!") {
        (b, "!END!")
    } else {
        (text, "")
    };
    let escaped_padding = body.contains("%3D");
    let body = body.replace("%3D", "=");
    let plain = STANDARD.decode(body.as_bytes()).ok()?;
    let s = String::from_utf8_lossy(&plain);
    let parts: Vec<&str> = s.split('|').collect();
    if parts.len() < 6 || !parts[0].starts_with("2.") || !parts[0][2..].bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let stats: Vec<&str> = parts[4].split(';').collect();
    let mut o = Map::new();
    o.insert("version".into(), json!(parts[0]));
    for (k, i) in STATS {
        o.insert(k.into(), stats.get(i).map(|t| num(t)).unwrap_or(Value::Null));
    }
    let mut b = Map::new();
    for (name, e) in BUILDINGS.iter().zip(parts[5].split(';')) {
        let t: Vec<&str> = e.split(',').collect();
        if t.len() < 4 {
            continue;
        }
        b.insert(name.to_string(), json!({ "owned": num(t[0]), "bought": num(t[1]), "level": num(t[3]) }));
    }
    o.insert("buildings".into(), Value::Object(b));
    let js = serde_json::to_vec_pretty(&json!({ "Cookie Clicker": Value::Object(o) })).ok()?;
    Some((Wrap { escaped_padding, end: end.to_string() }, plain, js))
}

pub fn pack(wrap: &Wrap, plain: &[u8], json: &[u8]) -> Vec<u8> {
    let s = String::from_utf8_lossy(plain).to_string();
    let Ok(v) = serde_json::from_slice::<Value>(json) else {
        return rewrap(wrap, plain);
    };
    let Some(o) = v.get("Cookie Clicker") else {
        return rewrap(wrap, plain);
    };
    let mut parts: Vec<String> = s.split('|').map(|x| x.to_string()).collect();
    if parts.len() < 6 {
        return rewrap(wrap, plain);
    }
    let mut stats: Vec<String> = parts[4].split(';').map(|x| x.to_string()).collect();
    for (k, i) in STATS {
        if let (Some(n), Some(t)) = (o.get(k).and_then(|x| x.as_f64()), stats.get_mut(i)) {
            *t = patch(t, Some(n));
        }
    }
    // keep "baked this run" at least what is in the bank
    if let (Ok(c), Ok(e)) = (stats[0].parse::<f64>(), stats[1].parse::<f64>()) {
        if c > e {
            stats[1] = stats[0].clone();
        }
    }
    parts[4] = stats.join(";");
    let mut ents: Vec<String> = parts[5].split(';').map(|x| x.to_string()).collect();
    for (name, e) in BUILDINGS.iter().zip(ents.iter_mut()) {
        let Some(b) = o.get("buildings").and_then(|b| b.get(*name)) else { continue };
        let mut t: Vec<String> = e.split(',').map(|x| x.to_string()).collect();
        if t.len() < 4 {
            continue;
        }
        let owned = b.get("owned").and_then(|x| x.as_f64());
        let bought = b.get("bought").and_then(|x| x.as_f64());
        let level = b.get("level").and_then(|x| x.as_f64());
        t[0] = patch(&t[0], owned);
        t[1] = patch(&t[1], bought);
        t[3] = patch(&t[3], level);
        // the game's counters of bought and highest owned never trail the amount owned
        if let (Ok(a), Ok(bt)) = (t[0].parse::<f64>(), t[1].parse::<f64>()) {
            if bt < a {
                t[1] = t[0].clone();
            }
        }
        if t.len() > 6 {
            if let (Ok(a), Ok(h)) = (t[0].parse::<f64>(), t[6].parse::<f64>()) {
                if h < a {
                    t[6] = t[0].clone();
                }
            }
        }
        *e = t.join(",");
    }
    parts[5] = ents.join(";");
    rewrap(wrap, parts.join("|").as_bytes())
}

fn rewrap(wrap: &Wrap, plain: &[u8]) -> Vec<u8> {
    let mut b = STANDARD.encode(plain);
    if wrap.escaped_padding {
        b = b.replace('=', "%3D");
    }
    b.push_str(&wrap.end);
    b.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut b = String::from("2.058||1;2;3;Bakery;seed;1,2|1101|1.5e+10;2.5e+10;10;1;5;0;1;2;3e+12;0;0;0;0;-1;2;0;0;4;5|");
        let line = "10,12,100,2,,0,10";
        b.push_str(&vec![line; 20].join(";"));
        b.push_str("|11|10|1;");
        STANDARD.encode(b).into_bytes()
    }

    #[test]
    fn unchanged_text_comes_back_identical() {
        let file = sample();
        let (wrap, plain, js) = open(&file).unwrap();
        assert_eq!(pack(&wrap, &plain, &js), file);
    }

    #[test]
    fn edits_patch_the_text() {
        let file = sample();
        let (wrap, plain, js) = open(&file).unwrap();
        let mut v: Value = serde_json::from_slice(&js).unwrap();
        v["Cookie Clicker"]["cookies"] = json!(1e30);
        v["Cookie Clicker"]["buildings"]["Farm"]["owned"] = json!(500);
        let out = pack(&wrap, &plain, &serde_json::to_vec(&v).unwrap());
        let (_, plain2, js2) = open(&out).unwrap();
        let v2: Value = serde_json::from_slice(&js2).unwrap();
        assert_eq!(v2["Cookie Clicker"]["cookies"], json!(1e30));
        assert_eq!(v2["Cookie Clicker"]["buildings"]["Farm"]["owned"], json!(500));
        assert!(String::from_utf8_lossy(&plain2).contains("1e+30"));
        assert_eq!(v2["Cookie Clicker"]["buildings"]["Farm"]["bought"], json!(500));
    }
}
