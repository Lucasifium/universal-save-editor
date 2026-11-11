//! Console saves opened through their patch file's own codes: the required "Decrypt …",
//! "Decompress …" (and similar) codes run when the save opens, and the rest of its required
//! codes (encrypt, compress, checksums) run again when it is saved, the way Apollo applies
//! a patch.

use super::{db, targets, Code, Kind, Session};

/// Names of the codes that unpack a save when it opens.
fn opens(c: &Code) -> bool {
    let n = c.name.trim_start().to_ascii_lowercase();
    n.starts_with("read ") || ["decrypt", "decompress", "unpack", "extract"].iter().any(|w| n.split(|c: char| !c.is_ascii_alphanumeric()).any(|x| x == *w))
}

/// A code that reads something (a key) from a file before the save itself is unpacked.
fn reads(c: &Code) -> bool {
    c.name.trim_start().to_ascii_lowercase().starts_with("read ")
}

fn names_file(c: &Code, file: &str) -> bool {
    !c.file.is_empty() && targets(c, file)
}

struct Plan {
    codes: Vec<Code>,
    open: Vec<usize>,
    close: Vec<usize>,
    big_endian: bool,
}

fn plan(e: &db::Entry, file: &str) -> Option<Plan> {
    let codes = e.codes();
    let req: Vec<usize> = (0..codes.len()).filter(|&i| codes[i].required && !codes[i].empty()).collect();
    // A code that reads another file first (a key file, say) can't run on this one alone.
    if req.iter().any(|&i| reads(&codes[i]) && !targets(&codes[i], file)) {
        return None;
    }
    let mine: Vec<usize> = req.into_iter().filter(|&i| targets(&codes[i], file)).collect();
    if mine.is_empty() || !mine.iter().any(|&i| names_file(&codes[i], file)) || mine.iter().any(|&i| codes[i].kind == Kind::Python) {
        return None;
    }
    let (open, close): (Vec<usize>, Vec<usize>) = mine.into_iter().partition(|&i| opens(&codes[i]));
    if close.is_empty() {
        return None;
    }
    Some(Plan { codes, open, close, big_endian: e.big_endian() })
}

/// What a save opened with: its patch file, and the unpacked stream it shows (if any).
#[derive(Clone, Debug, PartialEq)]
pub struct Opened {
    pub id: String,
    pub platform: String,
    pub title: String,
    pub var: Option<String>,
    /// The unpacked data, with `derived` zeroed.
    pub plain: Vec<u8>,
    /// Bytes of the unpacked data that packing writes (checksums and hashes kept inside it).
    /// They show as zeros and are filled in on save, like any checksum the editor keeps.
    pub derived: Vec<usize>,
}

/// Most bytes a patch file's packing codes may write inside the unpacked data.
const MAX_DERIVED: usize = 256;

/// Which bytes of `plain` the packing codes rewrite: change one byte at a time and see what
/// else changes when the data is packed and unpacked again.
fn derived(p: &Plan, original: &[u8], var: &Option<String>, plain: &[u8]) -> Option<Vec<usize>> {
    let mut out: Vec<usize> = vec![];
    for at in [plain.len() / 4, plain.len() / 2, plain.len() * 3 / 4] {
        let mut t = plain.to_vec();
        t[at] ^= 0x5A;
        let packed = close_with(p, original, var, &t).ok()?;
        let back = run_open(p, &packed).map(|(s, d, v)| plain_of(&s, d, &v))?;
        if back.len() != t.len() || back[at] != t[at] {
            return None;
        }
        out.extend((0..t.len()).filter(|&i| back[i] != t[i]));
    }
    out.sort_unstable();
    out.dedup();
    (out.len() <= MAX_DERIVED).then_some(out)
}

fn run_open(p: &Plan, data: &[u8]) -> Option<(Session, Vec<u8>, Option<String>)> {
    let mut s = Session::new(p.big_endian);
    let mut d = data.to_vec();
    for &i in &p.open {
        s.apply(&mut d, &p.codes[i], &[]).ok()?;
    }
    let var = s.vars.iter().find(|v| v.name.to_ascii_lowercase().starts_with("~extracted\\") && v.data.is_some()).map(|v| v.name.clone());
    Some((s, d, var))
}

fn plain_of(s: &Session, d: Vec<u8>, var: &Option<String>) -> Vec<u8> {
    match var {
        Some(v) => s.var(v).and_then(|v| v.data.clone()).unwrap_or_default(),
        None => d,
    }
}

fn close_with(p: &Plan, original: &[u8], var: &Option<String>, plain: &[u8]) -> Result<Vec<u8>, String> {
    let (mut s, mut d, _) = run_open(p, original).ok_or("the original save no longer unpacks")?;
    match var {
        Some(name) => {
            let v = s.vars.iter_mut().find(|v| &v.name == name).ok_or("the unpacked data is missing")?;
            v.len = plain.len();
            v.data = Some(plain.to_vec());
        }
        None => d = plain.to_vec(),
    }
    for &i in &p.close {
        s.apply(&mut d, &p.codes[i], &[]).map_err(|e| format!("{}: {e}", p.codes[i].name))?;
    }
    Ok(d)
}

/// Opens a console save whose folder or file name carries a title ID with a patch file
/// that unpacks it. The result is only used when packing it again gives back what it read.
pub fn open(path: &str, data: &[u8]) -> Option<Opened> {
    let path = path.replace('\\', "/");
    let file = path.rsplit('/').next().unwrap_or_default();
    for id in db::ids_in(&path) {
        for e in db::find(&id) {
            let Some(p) = plan(e, file) else { continue };
            let Some((s, d, var)) = run_open(&p, data) else { continue };
            let plain = plain_of(&s, d, &var);
            if plain.is_empty() {
                continue;
            }
            let Ok(again) = close_with(&p, data, &var, &plain) else { continue };
            let ok = if p.open.is_empty() {
                // Checksums only: they must already be right on the save as it is.
                again == data
            } else {
                plain != data && run_open(&p, &again).map(|(s, d, v)| plain_of(&s, d, &v)).as_deref() == Some(&plain[..])
            };
            if !ok {
                continue;
            }
            let Some(derived) = derived(&p, data, &var, &plain) else { continue };
            let mut plain = plain;
            for &i in &derived {
                plain[i] = 0;
            }
            return Some(Opened { id: e.id.clone(), platform: e.platform.clone(), title: e.title.clone(), var, plain, derived });
        }
    }
    None
}

/// Packs edited data again: the original is unpacked once more (for the codes' state),
/// the edit is put in, and the packing codes run.
pub fn close(id: &str, path: &str, var: &Option<String>, original: &[u8], plain: &[u8]) -> Result<Vec<u8>, String> {
    let path = path.replace('\\', "/");
    let file = path.rsplit('/').next().unwrap_or_default();
    let e = db::find(id).into_iter().find(|e| plan(e, file).is_some()).ok_or("the save's patch file is missing")?;
    close_with(&plan(e, file).unwrap(), original, var, plain)
}

#[cfg(test)]
mod tests {
    /// Lists the games whose saves open through their patch file (for the README's count).
    #[test]
    #[ignore]
    fn list_unpackable() {
        let mut n = 0;
        for e in super::db::all() {
            let codes = e.codes();
            let files: std::collections::BTreeSet<String> =
                codes.iter().filter(|c| c.required && !c.file.is_empty()).map(|c| c.file.rsplit(['\\', '/']).next().unwrap_or("").to_string()).collect();
            if let Some(f) = files.iter().find(|f| super::plan(e, f).is_some_and(|p| !p.open.is_empty())) {
                n += 1;
                println!("{} {} {} ({f})", e.platform, e.id, e.title);
            }
        }
        println!("{n} patch files unpack a save");
    }
}
