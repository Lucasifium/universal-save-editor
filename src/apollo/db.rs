//! The bundled patch database: apollo-patches (GPL-3.0-or-later, Damian Parrino and
//! contributors; see THIRD_PARTY_NOTICES), about 2,400 `.savepatch` files packed with xz.
//!
//! `patches.xz` holds, for each file in path order: a u16 name length, the name
//! (`PS2/SCES50003.savepatch`), a u32 text length and the text, all little-endian. It is
//! rebuilt from a checkout of apollo-patches with a few lines of script (see the module tests
//! for the reader it has to match).

use std::sync::OnceLock;

static PACKED: &[u8] = include_bytes!("patches.xz");

pub struct Entry {
    /// DC, PS1, PS2, PS3, PS4, PSP or PSV.
    pub platform: String,
    /// The title ID the file is named after (`SCES50003`).
    pub id: String,
    pub title: String,
    pub text: String,
    /// The file is single-byte text, not UTF-8.
    pub latin1: bool,
}

impl Entry {
    /// PS3 saves are big-endian; every other platform here is little-endian.
    pub fn big_endian(&self) -> bool {
        self.platform == "PS3"
    }

    pub fn codes(&self) -> Vec<super::Code> {
        super::parse(&self.text, self.latin1)
    }
}

fn unpack(raw: &[u8]) -> Vec<Entry> {
    let mut out = vec![];
    let mut at = 0;
    while at + 2 <= raw.len() {
        let n = u16::from_le_bytes([raw[at], raw[at + 1]]) as usize;
        at += 2;
        let Some(name) = raw.get(at..at + n).map(|b| String::from_utf8_lossy(b).into_owned()) else { break };
        at += n;
        let Some(len) = raw.get(at..at + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize) else { break };
        at += 4;
        let Some(body) = raw.get(at..at + len) else { break };
        at += len;
        let (text, latin1) = match std::str::from_utf8(body) {
            Ok(t) => (t.to_string(), false),
            Err(_) => (body.iter().map(|&b| b as char).collect(), true),
        };
        let (platform, file) = name.split_once('/').unwrap_or(("", &name));
        let id = file.trim_end_matches(".savepatch").to_string();
        let (_, mut title) = super::header(&text);
        if title.is_empty() || title.eq_ignore_ascii_case(&id) {
            title = id.clone();
        }
        out.push(Entry { platform: platform.to_string(), id, title, text, latin1 });
    }
    out
}

/// Every patch file (unpacked the first time it's asked for).
pub fn all() -> &'static [Entry] {
    static DB: OnceLock<Vec<Entry>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut raw = vec![];
        let _ = lzma_rs::xz_decompress(&mut std::io::BufReader::new(PACKED), &mut raw);
        unpack(&raw)
    })
}

/// Whether a title ID is a PS3 game's (big-endian saves): disc IDs start with B (BLUS,
/// BCES…), store ones are NP?B (NPUB, NPEB…).
pub fn ps3_id(id: &str) -> bool {
    let id: Vec<u8> = id.bytes().filter(u8::is_ascii_alphabetic).map(|b| b.to_ascii_uppercase()).take(4).collect();
    id.first() == Some(&b'B') || (id.starts_with(b"NP") && id.get(3) == Some(&b'B'))
}

fn norm(id: &str) -> String {
    id.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_uppercase()
}

/// The patch files for a title ID (`SCES-50003`, `BLUS30490`…).
pub fn find(id: &str) -> Vec<&'static Entry> {
    let id = norm(id);
    all().iter().filter(|e| norm(&e.id) == id).collect()
}

/// Title IDs that appear in a save's path or name (four letters and five digits, as in
/// `BESCES-50003` or `CUSA01615`), with a patch file for them.
pub fn ids_in(path: &str) -> Vec<String> {
    let b = path.as_bytes();
    let mut out: Vec<String> = vec![];
    for i in 0..b.len() {
        if i + 9 > b.len() || !b[i..i + 4].iter().all(|c| c.is_ascii_alphabetic()) {
            continue;
        }
        let mut j = i + 4;
        if matches!(b.get(j), Some(b'-' | b'_')) {
            j += 1;
        }
        if j + 5 <= b.len() && b[j..j + 5].iter().all(|c| c.is_ascii_digit()) && !b.get(j + 5).is_some_and(|c| c.is_ascii_digit()) {
            let id = format!("{}{}", &path[i..i + 4], &path[j..j + 5]).to_ascii_uppercase();
            if !out.contains(&id) && !find(&id).is_empty() {
                out.push(id);
            }
        }
    }
    out
}

/// Patch files whose ID or game name contains `q` (case-insensitive), up to `max`.
pub fn search(q: &str, max: usize) -> Vec<&'static Entry> {
    let q = q.trim().to_lowercase();
    let qn = norm(&q);
    all()
        .iter()
        .filter(|e| q.is_empty() || e.title.to_lowercase().contains(&q) || (!qn.is_empty() && norm(&e.id).contains(&qn)))
        .take(max)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database() {
        let all = all();
        assert!(all.len() > 2000, "{}", all.len());
        let e = find("SCES-50003");
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].platform, "PS2");
        assert!(e[0].title.contains("Dead or Alive"));
        assert!(!e[0].codes().is_empty());
        assert_eq!(ids_in("C:/cards/BESCES-50003/BESCES-50003"), vec!["SCES50003".to_string()]);
        assert!(!search("dead or alive", 50).is_empty());
        assert!(ps3_id("BLUS30490") && ps3_id("NPEB00123") && !ps3_id("NPUH10001") && !ps3_id("CUSA01615"));
    }
}
