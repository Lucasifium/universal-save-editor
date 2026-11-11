//! Antimatter Dimensions exported saves: a text prefix naming the platform, a three letter
//! version, then the JSON compressed (zlib on PC, gzip on mobile), written as base64 with
//! `0` -> `0a`, `+` -> `0b`, `/` -> `0c`, no padding and, for PC version `AAB` and mobile,
//! a closing `EndOfSavefile`.
use base64::{engine::general_purpose::STANDARD, Engine};
use flate2::{
    bufread::{GzDecoder, ZlibDecoder},
    write::{GzEncoder, ZlibEncoder},
    Compression,
};
use std::io::{Read, Write};

const PREFIXES: [&str; 3] = [
    "AntimatterDimensionsSavefileFormat",
    "AntimatterDimensionsAndroidSaveFormat",
    "AntimatterDimensionsAppleSaveFormat",
];
const END: &str = "EndOfSavefile";
/// Largest JSON accepted, so a hostile file cannot inflate without limit.
const LIMIT: u64 = 256 * 1024 * 1024;

pub fn open(data: &[u8]) -> Option<(u8, String, Vec<u8>)> {
    let text = std::str::from_utf8(data).ok()?.trim();
    let kind = PREFIXES.iter().position(|p| text.starts_with(p))?;
    let rest = &text[PREFIXES[kind].len()..];
    let version = rest.get(..3)?;
    if !version.bytes().all(|b| b.is_ascii_uppercase()) {
        return None;
    }
    let body = rest[3..].strip_suffix(END).unwrap_or(&rest[3..]);
    let b64 = body
        .replace("0b", "+")
        .replace("0c", "/")
        .replace("0a", "0");
    let raw = STANDARD
        .decode(format!("{b64}{}", "=".repeat((4 - b64.len() % 4) % 4)))
        .ok()?;
    let mut out = Vec::new();
    let ok = if kind == 0 {
        ZlibDecoder::new(&raw[..]).take(LIMIT).read_to_end(&mut out)
    } else {
        GzDecoder::new(&raw[..]).take(LIMIT).read_to_end(&mut out)
    };
    ok.ok()?;
    Some((kind as u8, version.to_string(), out))
}

pub fn close(kind: u8, version: &str, plain: &[u8]) -> Result<Vec<u8>, String> {
    let e = |e: std::io::Error| e.to_string();
    let raw = if kind == 0 {
        let mut z = ZlibEncoder::new(Vec::new(), Compression::default());
        z.write_all(plain).map_err(e)?;
        z.finish().map_err(e)?
    } else {
        let mut z = GzEncoder::new(Vec::new(), Compression::default());
        z.write_all(plain).map_err(e)?;
        z.finish().map_err(e)?
    };
    let b64 = STANDARD
        .encode(raw)
        .trim_end_matches('=')
        .replace('0', "0a")
        .replace('+', "0b")
        .replace('/', "0c");
    let prefix = PREFIXES
        .get(kind as usize)
        .ok_or("unknown Antimatter Dimensions platform")?;
    let end = if kind != 0 || version == "AAB" {
        END
    } else {
        ""
    };
    Ok(format!("{prefix}{version}{b64}{end}").into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_every_platform() {
        let json = br#"{"antimatter":"10","version":25}"#;
        for (kind, version) in [(0u8, "AAB"), (0, "AAA"), (1, "AAA"), (2, "AAA")] {
            let text = close(kind, version, json).unwrap();
            let (k, v, plain) = open(&text).expect("opens");
            assert_eq!((k, v.as_str(), &plain[..]), (kind, version, &json[..]));
        }
        assert!(open(b"plain text").is_none());
    }
}
