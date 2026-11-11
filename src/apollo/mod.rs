//! Console cheat patches: the Apollo Save Tool patch format (`.savepatch`) and its two code
//! languages, Save Wizard / Game Genie codes and Bruteforce Save Data scripts, applied to a
//! decrypted console save (or an emulator's save). About 2,400 patch files from the
//! apollo-patches database ship with the editor.
//!
//! Ported from apollo-lib (`loader.c`, `patches.c`, `crc_util.c`, `decrypt.c`, `offzip.c`,
//! `packzip.c`, `wildcard.c`), GPL-3.0-or-later, Copyright (C) Damian Parrino ("bucanero");
//! the patch database is apollo-patches, GPL-3.0-or-later, by the same author and its
//! contributors. See THIRD_PARTY_NOTICES. Patches written as Python scripts need Apollo's
//! MicroPython runtime and are listed but can't be applied.

mod bsd;
pub mod console;
pub mod crypt;
pub mod db;
pub mod hash;
mod keys;
pub mod offzip;
mod sw;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    SaveWizard,
    Bsd,
    Python,
}

/// A `{TAG}` in a code and the values it can take: (value, name).
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    pub tag: String,
    pub values: Vec<(String, String)>,
}

#[derive(Clone, Debug)]
pub struct Code {
    pub name: String,
    /// The save file it targets (may have wildcards and a folder: `FOLDER\FILE`).
    pub file: String,
    pub kind: Kind,
    pub body: String,
    pub default_on: bool,
    pub alert: bool,
    pub required: bool,
    /// Starts a group (a heading) / belongs to one.
    pub parent: bool,
    pub child: bool,
    /// Forced byte order for Save Wizard codes: Some(true) big-endian.
    pub big_endian: Option<bool>,
    pub choices: Vec<Choice>,
    /// The patch file wasn't UTF-8: its quoted text is single bytes.
    latin1: bool,
}

impl Code {
    pub fn empty(&self) -> bool {
        self.body.trim().is_empty()
    }
}

// --- Wildcards and number parsing, as the C code reads them ---

/// `*` and `?` wildcards over the whole string; an empty string never matches.
pub(crate) fn glob_with(data: &str, mask: &str, icase: bool) -> bool {
    let (d, m) = (data.as_bytes(), mask.as_bytes());
    if d.is_empty() || m.is_empty() {
        return false;
    }
    let eq = |a: u8, b: u8| if icase { a.to_ascii_uppercase() == b.to_ascii_uppercase() } else { a == b };
    let (mut i, mut j) = (0, 0);
    let (mut star, mut mark) = (None, 0);
    while i < d.len() {
        if j < m.len() && (m[j] == b'?' || (m[j] != b'*' && eq(m[j], d[i]))) {
            i += 1;
            j += 1;
        } else if j < m.len() && m[j] == b'*' {
            star = Some(j);
            mark = i;
            j += 1;
        } else if let Some(s) = star {
            j = s + 1;
            mark += 1;
            i = mark;
        } else {
            return false;
        }
    }
    while j < m.len() && m[j] == b'*' {
        j += 1;
    }
    j == m.len()
}

pub(crate) fn glob(d: &str, m: &str) -> bool {
    glob_with(d, m, false)
}

pub(crate) fn glob_i(d: &str, m: &str) -> bool {
    glob_with(d, m, true)
}

/// `sscanf("%x")`: leading blanks, an optional sign and `0x`, then hex digits (wrapping to
/// 32 bits). None when no digit is there.
pub(crate) fn scan_hex(s: &str) -> Option<u32> {
    let s = s.trim_start();
    let (neg, s) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let b = s.as_bytes();
    let s = if b.len() > 2 && b[0] == b'0' && (b[1] | 0x20) == b'x' && b[2].is_ascii_hexdigit() { &s[2..] } else { s };
    let mut v: u64 = 0;
    let mut n = 0;
    for c in s.bytes() {
        let Some(x) = (c as char).to_digit(16) else { break };
        v = (v << 4 | x as u64) & 0xFFFF_FFFF_FFFF_FFFF;
        n += 1;
    }
    (n > 0).then(|| if neg { (v as u32).wrapping_neg() } else { v as u32 })
}

/// `sscanf("%" PRIx64)`.
pub(crate) fn scan_hex64(s: &str) -> Option<u64> {
    let s = s.trim_start();
    let b = s.as_bytes();
    let s = if b.len() > 2 && b[0] == b'0' && (b[1] | 0x20) == b'x' && b[2].is_ascii_hexdigit() { &s[2..] } else { s };
    let mut v: u64 = 0;
    let mut n = 0;
    for c in s.bytes() {
        let Some(x) = (c as char).to_digit(16) else { break };
        v = v.wrapping_shl(4) | x as u64;
        n += 1;
    }
    (n > 0).then_some(v)
}

/// `sscanf("%d")`.
pub(crate) fn scan_dec(s: &str) -> Option<i32> {
    let s = s.trim_start();
    let (neg, s) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let v = digits.parse::<i64>().unwrap_or(i64::MAX) as i32;
    Some(if neg { v.wrapping_neg() } else { v })
}

/// The hex number in `n` characters of `line` from `at` (0 if none).
pub(crate) fn hex_at(line: &str, at: usize, n: usize) -> u32 {
    line.get(at..(at + n).min(line.len())).and_then(scan_hex).unwrap_or(0)
}

// --- The .savepatch file ---

fn rtrim(s: &str) -> &str {
    s.trim_end_matches([' ', '\t'])
}

fn ends_with_i(a: &str, b: &str) -> Option<usize> {
    (a.len() >= b.len() && a.is_char_boundary(a.len() - b.len()) && a[a.len() - b.len()..].eq_ignore_ascii_case(b)).then(|| a.len() - b.len())
}

fn is_title(l: &str) -> bool {
    glob(l, "[*]") || glob(l, "; --- * ---") || glob_i(l, "GROUP:*")
}

fn ends_body(l: &str) -> bool {
    glob(l, "; --- * ---") || glob(l, ":*") || glob(l, "[*]") || glob(l, "{*}*{/*}") || glob_i(l, "PATH:*") || glob_i(l, "GROUP:*")
}

fn parse_choice(line: &str) -> Option<Choice> {
    // {TAG}value=name;value=name{/TAG}
    let close = line.find('}')?;
    let tag = format!("{{{}}}", &line[1..close]);
    let rest = &line[close + 1..];
    let end = rest.find('{')?;
    let list = &rest[..end];
    let width = tag.len();
    let mut values = vec![];
    let mut pending: Option<String> = None;
    let mut start = 0;
    let b = list.as_bytes();
    let mut x = 0;
    while x <= b.len() {
        if x == b.len() || b[x] == b'=' || b[x] == b';' {
            let piece = &list[start..x];
            if x < b.len() && b[x] == b'=' {
                let mut v: String = piece.to_string();
                if v.len() < width {
                    v = "0".repeat(width - v.len()) + &v;
                } else {
                    v.truncate(width);
                }
                pending = Some(v);
            } else if let Some(v) = pending.take() {
                values.push((v, piece.to_string()));
            }
            start = x + 1;
        }
        x += 1;
    }
    Some(Choice { tag, values })
}

/// Reads a patch file's codes (headings included), in order.
pub fn parse(text: &str, latin1: bool) -> Vec<Code> {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = text.split('\n').map(rtrim).filter(|l| !l.is_empty()).collect();
    let mut codes = vec![];
    let mut file = String::new();
    let mut defined: Vec<Choice> = vec![];
    let mut group = 0u8; // 0 none, 1 the next code starts a group, 2 inside one
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        i += 1;
        if glob(line, ":*") {
            file = line[1..].chars().take(255).collect();
            continue;
        }
        if glob(line, "{*}*{/*}") {
            if let Some(c) = parse_choice(line) {
                defined.push(c);
            }
            continue;
        }
        if glob_i(line, "PATH:*") || !is_title(line) {
            continue;
        }
        let mut code = Code {
            name: String::new(),
            file: file.clone(),
            kind: Kind::SaveWizard,
            body: String::new(),
            default_on: false,
            alert: false,
            required: false,
            parent: false,
            child: false,
            big_endian: None,
            choices: vec![],
            latin1,
        };
        let mut declared = None;
        let mut t: &str = line;
        // `at` is one character before the next token (the '[' at first, then each ':').
        let name: String = if glob_i(t, "*GROUP:\\*") {
            group = 0;
            String::new()
        } else if glob_i(t, "[GROUP:*") {
            group = 1;
            t[7..].to_string()
        } else if glob(t, "; --- * ---") || glob_i(t, "GROUP:*") {
            group = 1;
            t.get(6..).unwrap_or("").to_string()
        } else {
            loop {
                let rest = &t[1..];
                let n = if glob_i(rest, "DEFAULT:*") {
                    code.default_on = true;
                    8
                } else if glob_i(rest, "INFO:*") {
                    code.alert = true;
                    5
                } else if glob_i(rest, "PYTHON:*") {
                    declared = Some(Kind::Python);
                    7
                } else if glob_i(rest, "SW:*") {
                    declared = Some(Kind::SaveWizard);
                    3
                } else if glob_i(rest, "BSD:*") {
                    declared = Some(Kind::Bsd);
                    4
                } else if glob_i(rest, "LE:*") {
                    code.big_endian = Some(false);
                    3
                } else if glob_i(rest, "BE:*") {
                    code.big_endian = Some(true);
                    3
                } else {
                    break;
                };
                t = &t[n..];
            }
            t[1..].to_string()
        };
        let mut name = name;
        if let Some(e) = name.rfind(']') {
            name.truncate(e);
        }
        if let Some(e) = ends_with_i(&name, " ---") {
            name.truncate(e);
        }
        code.required = glob_i(&name, "*(REQUIRED)*");
        code.name = name;
        match group {
            1 => {
                code.parent = true;
                group = 2;
            }
            2 => code.child = true,
            _ => {}
        }
        // The body: every line up to the next title, file or option line.
        let mut inferred = None;
        let mut tags: Vec<Choice> = vec![];
        while i < lines.len() && !ends_body(lines[i]) {
            let l = lines[i];
            i += 1;
            if l.starts_with(';') {
                continue;
            }
            code.body.push_str(l);
            code.body.push('\n');
            if inferred != Some(Kind::Bsd) {
                inferred = Some(if glob(l, "???????? ????????") { Kind::SaveWizard } else { Kind::Bsd });
            }
            for c in &defined {
                if l.contains(&c.tag) && !tags.iter().any(|t| t.tag == c.tag) {
                    tags.push(c.clone());
                }
            }
        }
        code.choices = tags;
        code.kind = declared.or(inferred).unwrap_or(Kind::SaveWizard);
        codes.push(code);
    }
    codes
}

/// A patch file's first comment lines: its title ID and game name.
pub fn header(text: &str) -> (String, String) {
    let mut it = text.lines().take(3).filter_map(|l| l.trim().strip_prefix(';')).map(|l| l.trim().to_string());
    (it.next().unwrap_or_default(), it.next().unwrap_or_default())
}

// --- Applying codes ---

/// A BSD script variable: its bytes in file order, or None once cleared.
#[derive(Clone, Debug)]
pub(crate) struct Var {
    pub name: String,
    pub data: Option<Vec<u8>>,
    pub len: usize,
}

/// One run of the selected codes over a save: script variables and unpacked streams carry
/// from one code to the next, as they do in Apollo.
pub struct Session {
    pub(crate) vars: Vec<Var>,
    pub(crate) zips: Vec<offzip::Found>,
    /// The platform's byte order for Save Wizard codes (PS3 is big-endian, the rest little).
    pub big_endian: bool,
}

const EXTRACTED: &str = "~extracted\\";

impl Session {
    pub fn new(big_endian: bool) -> Self {
        Session { vars: vec![], zips: vec![], big_endian }
    }

    /// Starts from data already unpacked into `name` (a save the editor opened unpacked):
    /// codes for that stream then work on it directly.
    pub fn seed(&mut self, name: &str, data: Vec<u8>) {
        self.vars.push(Var { name: name.to_string(), len: data.len(), data: Some(data) });
    }

    /// The bytes of an unpacked stream.
    pub fn unpacked(&self, name: &str) -> Option<&[u8]> {
        self.var(name).and_then(|v| v.data.as_deref())
    }

    pub(crate) fn var(&self, name: &str) -> Option<&Var> {
        self.vars.iter().find(|v| v.name == name)
    }

    /// The code's text with each `{TAG}` replaced by the chosen value (`picks[i]` for
    /// `choices[i]`; the first value when not given).
    pub fn text(code: &Code, picks: &[usize]) -> String {
        let mut t = code.body.clone();
        for (i, c) in code.choices.iter().enumerate() {
            if let Some((v, _)) = c.values.get(picks.get(i).copied().unwrap_or(0)) {
                t = t.replace(&c.tag, v);
            }
        }
        t
    }

    /// Applies one code. On an error the save is left as it was.
    pub fn apply(&mut self, data: &mut Vec<u8>, code: &Code, picks: &[usize]) -> Result<(), String> {
        let text = Self::text(code, picks);
        let be = code.big_endian.unwrap_or(self.big_endian);
        let run = |s: &mut Session, buf: &mut Vec<u8>| -> Result<(), String> {
            match code.kind {
                Kind::Python => Err("this patch is a Python script, which needs Apollo itself to run".into()),
                Kind::SaveWizard => sw::run(buf, &text, be),
                Kind::Bsd => bsd::run(s, buf, &text, be, code.latin1),
            }
        };
        if code.file.len() > EXTRACTED.len() && code.file[..EXTRACTED.len()].eq_ignore_ascii_case(EXTRACTED) {
            let Some(at) = self.vars.iter().position(|v| v.name.eq_ignore_ascii_case(&code.file)) else {
                return Err(format!("{} hasn't been unpacked (its \"decompress\" code has to run first)", code.file));
            };
            let mut buf = self.vars[at].data.clone().unwrap_or_default();
            run(self, &mut buf)?;
            self.vars[at].len = buf.len();
            self.vars[at].data = Some(buf);
            return Ok(());
        }
        let mut buf = data.clone();
        run(self, &mut buf)?;
        if buf.is_empty() {
            return Err("the code left nothing of the file".into());
        }
        *data = buf;
        Ok(())
    }
}

/// Whether a code's target file pattern (`FOLDER\FILE`, wildcards allowed) fits a save file
/// name.
pub fn targets(code: &Code, file_name: &str) -> bool {
    if code.file.is_empty() || code.file.len() >= EXTRACTED.len() && code.file[..EXTRACTED.len()].eq_ignore_ascii_case(EXTRACTED) {
        return true;
    }
    let pat = code.file.rsplit(['\\', '/']).next().unwrap_or(&code.file);
    glob_i(file_name, pat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcards_and_numbers() {
        assert!(glob("set [b]:read(0x10,4)", "set *:*"));
        assert!(glob_i("WRITE at 1:2", "write *:*"));
        assert!(!glob("", "*"));
        assert!(glob("abc", "a?c"));
        assert!(!glob("abd", "a?c"));
        assert!(glob("x.DAT", "*.DAT"));
        assert_eq!(scan_hex(" 0x1F zz"), Some(0x1F));
        assert_eq!(scan_hex("(10)"), None);
        assert_eq!(scan_hex("FFFFFFFFF"), Some(0xFFFF_FFFF));
        assert_eq!(scan_dec(" -12x"), Some(-12));
    }

    #[test]
    fn reads_patch_files() {
        let text = "; SCES50003\r\n; A Game\r\n:BESCES-50003\r\n{AMT}0063=99;03E7=999{/AMT}\r\n[GROUP:Main]\r\n[DEFAULT:Gold]\r\n20000010 000{AMT}\r\n[Unlock]\t\r\nset [b]:read(0x10,1)\r\n; a comment\r\nwrite at 0x10:[b]\r\n[Group:\\]\r\n[INFO:BE:Note]\r\n[Checksum (Required)]\r\n00000000 00000001\r\n";
        let c = parse(text, false);
        let names: Vec<&str> = c.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Main", "Gold", "Unlock", "", "Note", "Checksum (Required)"]);
        assert!(c[0].parent && c[1].child && c[2].child && !c[4].child);
        assert!(c[1].default_on && c[4].alert && c[5].required);
        assert_eq!(c[1].kind, Kind::SaveWizard);
        assert_eq!(c[2].kind, Kind::Bsd);
        assert_eq!(c[2].body, "set [b]:read(0x10,1)\nwrite at 0x10:[b]\n");
        assert_eq!(c[4].big_endian, Some(true));
        assert_eq!(c[1].file, "BESCES-50003");
        assert_eq!(c[1].choices[0].values, vec![("00063".to_string(), "99".to_string()), ("003E7".to_string(), "999".to_string())]);
        assert_eq!(Session::text(&c[1], &[1]), "20000010 000003E7\n");
        assert_eq!(header(text), ("SCES50003".into(), "A Game".into()));
    }

    /// apollo-lib's golden corpus: every code of twelve real patch files applied to the same
    /// 2 MiB pseudo-random buffer must give the same bytes (hash) as the original engine.
    #[test]
    fn matches_apollo_corpus() {
        let mut base = vec![0u8; 0x20_0000];
        let mut x: u32 = 0x1234_abcd;
        for b in base.iter_mut() {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            *b = (x >> 24) as u8;
        }
        let fnv = |d: &[u8]| d.iter().fold(1469598103934665603u64, |h, &b| (h ^ b as u64).wrapping_mul(1099511628211));
        let golden = include_str!("corpus_le.txt");
        let mut files: Vec<&str> = golden.lines().filter_map(|l| l.split('#').next()).collect();
        files.dedup();
        let mut got = String::new();
        for f in files {
            let e = db::find(f.trim_end_matches(".savepatch"));
            assert_eq!(e.len(), 1, "{f}");
            for (i, mut c) in e[0].codes().into_iter().enumerate() {
                let ty = match c.kind {
                    Kind::SaveWizard => 1,
                    Kind::Bsd => 2,
                    Kind::Python => 3,
                };
                let head = format!("{f}#{}\t{ty}\t", i + 1);
                if c.kind == Kind::Python {
                    got += &format!("{head}skip-python\t-\t-\n");
                    continue;
                }
                if c.body.is_empty() {
                    got += &format!("{head}empty\t-\t-\n");
                    continue;
                }
                c.file.clear();
                let mut d = base.clone();
                match Session::new(false).apply(&mut d, &c, &[]) {
                    Ok(()) => got += &format!("{head}ok\t{}\t{:016x}\n", d.len(), fnv(&d)),
                    Err(_) => got += &format!("{head}noop\t0\t-\n"),
                }
            }
        }
        let diff: Vec<(&str, &str)> = golden.lines().zip(got.lines()).filter(|(a, b)| a != b).collect();
        assert!(diff.is_empty() && golden.lines().count() == got.lines().count(), "{diff:#?}");
    }
}
