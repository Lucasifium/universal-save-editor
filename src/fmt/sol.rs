//! Flash and Adobe AIR shared objects (`.sol`): the saves of Flash games and of many AIR games
//! on PC (AMF0 or AMF3 inside a "TCSO" header).
//!
//! Read and written with Ruffle's `flash-lso` (MIT). The values open as JSON; saving parses the
//! original file again and puts the edited numbers, flags and text back into it with their
//! own types (an AMF3 integer stays an integer, a Number stays a Number), so everything the
//! JSON can't show (dates, byte arrays, class names, references) is kept as it was.

use flash_lso::types::{Element, Lso, Value};
use serde_json::{Map, Number, Value as Json};

pub fn looks(d: &[u8]) -> bool {
    d.len() > 16 && d[0] == 0 && d[1] == 0xBF && &d[6..10] == b"TCSO"
}

/// Runs the library where a panic can be caught (it panics on some unusual files). The
/// browser version can't catch panics, so it leaves `.sol` files to the hex view.
fn guarded<T>(f: impl FnOnce() -> Option<T> + std::panic::UnwindSafe) -> Option<T> {
    if cfg!(target_arch = "wasm32") {
        return None;
    }
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let r = std::panic::catch_unwind(f).ok().flatten();
    std::panic::set_hook(hook);
    r
}

fn parse(d: &[u8]) -> Option<Lso> {
    let d = d.to_vec();
    guarded(move || flash_lso::read::Reader::default().parse(&d).ok())
}

fn write(lso: &Lso) -> Option<Vec<u8>> {
    let mut l = lso.clone();
    guarded(move || flash_lso::write::write_to_bytes(&mut l).ok())
}

fn num(f: f64) -> Json {
    if f.is_nan() {
        return Json::String("NaN".into());
    }
    if f.is_infinite() {
        return Json::String(if f > 0.0 { "Infinity" } else { "-Infinity" }.into());
    }
    if f.fract() == 0.0 && f.abs() < 9.0e15 {
        Json::from(f as i64)
    } else {
        Number::from_f64(f).map(Json::Number).unwrap_or(Json::Null)
    }
}

fn elements(els: &[Element]) -> Map<String, Json> {
    els.iter()
        .map(|e| (e.name.clone(), to_json(&e.value)))
        .collect()
}

fn to_json(v: &Value) -> Json {
    match v {
        Value::Number(f) => num(*f),
        Value::Integer(i) => Json::from(*i),
        Value::Bool(b) => Json::Bool(*b),
        Value::String(s) => Json::String(s.clone()),
        Value::AMF3(inner) => to_json(inner),
        Value::Object { data, .. } => Json::Object(elements(&data.elements)),
        Value::ECMAArray { data, .. } => {
            let mut m = Map::new();
            for (i, d) in data.dense.iter().enumerate() {
                m.insert(i.to_string(), to_json(d));
            }
            m.extend(elements(&data.elements));
            Json::Object(m)
        }
        Value::StrictArray { values, .. } => Json::Array(values.iter().map(to_json).collect()),
        Value::VectorInt(v) => Json::Array(v.values.iter().map(|x| Json::from(*x)).collect()),
        Value::VectorUInt(v) => Json::Array(v.values.iter().map(|x| Json::from(*x)).collect()),
        Value::VectorDouble(v) => Json::Array(v.values.iter().map(|x| num(*x)).collect()),
        Value::VectorObject { data, .. } => Json::Array(data.values.iter().map(to_json).collect()),
        Value::Custom(c) => {
            let mut m = elements(&c.elements);
            m.extend(elements(&c.dynamic_elements));
            Json::Object(m)
        }
        Value::Dictionary { data, .. } => Json::Array(
            data.elements
                .iter()
                .map(|e| Json::Array(vec![to_json(&e.key), to_json(&e.value)]))
                .collect(),
        ),
        Value::Null | Value::Undefined => Json::Null,
        other => Json::String(placeholder(other)),
    }
}

/// How a value the editor doesn't edit is shown.
fn placeholder(v: &Value) -> String {
    match v {
        Value::Date { time, .. } => format!("\u{2039}date {time}\u{203A}"),
        Value::ByteArray(b) => format!("\u{2039}{} bytes\u{203A}", b.len()),
        Value::XML { value, .. } => format!("\u{2039}XML, {} characters\u{203A}", value.len()),
        _ => "\u{2039}kept as it is\u{203A}".into(),
    }
}

fn set_elements(els: &mut [Element], j: &Map<String, Json>, path: &str) -> Result<(), String> {
    for e in els {
        if let Some(n) = j.get(&e.name) {
            apply(&mut e.value, n, &format!("{path}/{}", e.name))?;
        }
    }
    Ok(())
}

fn as_f64(j: &Json, path: &str) -> Result<f64, String> {
    match j.as_str() {
        Some("NaN") => return Ok(f64::NAN),
        Some("Infinity") => return Ok(f64::INFINITY),
        Some("-Infinity") => return Ok(f64::NEG_INFINITY),
        _ => {}
    }
    j.as_f64().ok_or_else(|| format!("{path} must be a number"))
}

/// Puts the edited JSON back into `v`, keeping each value's own type.
fn apply(v: &mut Value, j: &Json, path: &str) -> Result<(), String> {
    match v {
        Value::Number(f) => set_f64(f, as_f64(j, path)?),
        Value::Integer(i) => {
            let n = j
                .as_i64()
                .ok_or_else(|| format!("{path} must be a whole number"))?;
            // AMF3 integers hold 29 bits; a bigger number is stored as a Number instead.
            if (-(1 << 28)..(1 << 28)).contains(&n) {
                *i = n as i32;
            } else {
                *v = Value::Number(n as f64);
            }
        }
        Value::Bool(b) => {
            *b = j
                .as_bool()
                .ok_or_else(|| format!("{path} must be true or false"))?
        }
        Value::String(s) => {
            *s = j
                .as_str()
                .ok_or_else(|| format!("{path} must be text"))?
                .to_string()
        }
        Value::AMF3(inner) => apply(inner, j, path)?,
        Value::Object { data, .. } => set_elements(
            &mut data.elements,
            j.as_object()
                .ok_or_else(|| format!("{path} must stay an object"))?,
            path,
        )?,
        Value::ECMAArray { data, .. } => {
            let m = j
                .as_object()
                .ok_or_else(|| format!("{path} must stay an object"))?;
            for (i, d) in data.dense.iter_mut().enumerate() {
                if let Some(n) = m.get(&i.to_string()) {
                    apply(d, n, &format!("{path}/{i}"))?;
                }
            }
            set_elements(&mut data.elements, m, path)?;
        }
        Value::StrictArray { values, .. } => apply_list(values, j, path)?,
        Value::VectorObject { data, .. } => apply_list(&mut data.values, j, path)?,
        Value::VectorInt(vec) => {
            for (x, n) in vec.values.iter_mut().zip(list(j, path)?) {
                *x = n
                    .as_i64()
                    .and_then(|n| i32::try_from(n).ok())
                    .ok_or_else(|| format!("{path}: whole numbers only"))?;
            }
        }
        Value::VectorUInt(vec) => {
            for (x, n) in vec.values.iter_mut().zip(list(j, path)?) {
                *x = n
                    .as_u64()
                    .and_then(|n| u32::try_from(n).ok())
                    .ok_or_else(|| format!("{path}: whole numbers from 0 only"))?;
            }
        }
        Value::VectorDouble(vec) => {
            for (x, n) in vec.values.iter_mut().zip(list(j, path)?) {
                set_f64(x, as_f64(n, path)?);
            }
        }
        Value::Custom(c) => {
            let m = j
                .as_object()
                .ok_or_else(|| format!("{path} must stay an object"))?;
            set_elements(&mut c.elements, m, path)?;
            set_elements(&mut c.dynamic_elements, m, path)?;
        }
        Value::Dictionary { data, .. } => {
            for (e, n) in data.elements.iter_mut().zip(list(j, path)?) {
                if let Some(pair) = n.as_array().filter(|p| p.len() == 2) {
                    apply(&mut e.value, &pair[1], path)?;
                }
            }
        }
        Value::Null | Value::Undefined => {
            if !j.is_null() {
                return Err(format!(
                    "{path} is empty in the save and can't be given a value"
                ));
            }
        }
        other => {
            if j.as_str() != Some(&placeholder(other)) {
                return Err(format!("{path} can't be edited"));
            }
        }
    }
    Ok(())
}

/// Leaves a number alone unless it really changed (keeps the exact bits of a NaN).
fn set_f64(x: &mut f64, n: f64) {
    if !(*x == n || (x.is_nan() && n.is_nan())) {
        *x = n;
    }
}

fn list<'a>(j: &'a Json, path: &str) -> Result<&'a Vec<Json>, String> {
    j.as_array()
        .ok_or_else(|| format!("{path} must stay a list"))
}

fn apply_list(values: &mut [Value], j: &Json, path: &str) -> Result<(), String> {
    for (i, (x, n)) in values.iter_mut().zip(list(j, path)?).enumerate() {
        apply(x, n, &format!("{path}/{i}"))?;
    }
    Ok(())
}

/// The values of a `.sol` file as JSON.
pub fn open(d: &[u8]) -> Option<Vec<u8>> {
    if !looks(d) {
        return None;
    }
    let lso = parse(d)?;
    // Only files the library writes back exactly are opened for editing.
    if write(&lso)? != d {
        return None;
    }
    serde_json::to_vec_pretty(&Json::Object(elements(&lso.body))).ok()
}

/// `original` with the edited values from `json`.
pub fn save(original: &[u8], json: &[u8]) -> Result<Vec<u8>, String> {
    let mut lso = parse(original).ok_or("the original .sol file no longer parses")?;
    let j: Json = serde_json::from_slice(json).map_err(|e| e.to_string())?;
    set_elements(
        &mut lso.body,
        j.as_object().ok_or("the values must stay an object")?,
        "",
    )?;
    write(&lso).ok_or_else(|| "the edited .sol file couldn't be written".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use flash_lso::types::AMFVersion;

    #[test]
    fn edit_amf0_and_amf3() {
        for version in [AMFVersion::AMF0, AMFVersion::AMF3] {
            let int = if version == AMFVersion::AMF3 {
                Value::Integer(5)
            } else {
                Value::Number(5.0)
            };
            let mut lso = Lso::new(
                vec![
                    Element::new("coins", int),
                    Element::new("name", Value::String("Ann".into())),
                    Element::new("hard", Value::Bool(false)),
                ],
                "save",
                version,
            );
            let file = flash_lso::write::write_to_bytes(&mut lso).unwrap();
            assert!(looks(&file));
            let json = open(&file).expect("opens");
            let v: Json = serde_json::from_slice(&json).unwrap();
            assert_eq!(v["coins"], 5);
            let out = save(&file, br#"{"coins": 99999, "name": "Bo", "hard": true}"#).unwrap();
            let v: Json = serde_json::from_slice(&open(&out).unwrap()).unwrap();
            assert_eq!(
                v,
                serde_json::json!({"coins": 99999, "name": "Bo", "hard": true})
            );
            assert_eq!(save(&file, &json).unwrap(), file);
        }
    }

    /// SOL_SAMPLES = a folder of .sol files (e.g. rust-flash-lso's tests/sol): each must open
    /// and write back unchanged.
    #[test]
    #[ignore]
    fn sample_files() {
        let dir = std::env::var("SOL_SAMPLES").expect("SOL_SAMPLES");
        let (mut ok, mut skipped) = (0, vec![]);
        for e in std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .filter(|e| e.path().is_file())
        {
            let d = std::fs::read(e.path()).unwrap();
            match open(&d) {
                Some(json) => {
                    assert_eq!(save(&d, &json).unwrap(), d, "{:?}", e.path());
                    ok += 1;
                }
                None => skipped.push(e.file_name().to_string_lossy().to_string()),
            }
        }
        println!("{ok} opened and written back unchanged; not opened: {skipped:?}");
    }
}
