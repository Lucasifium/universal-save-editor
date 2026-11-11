//! Bruteforce Save Data scripts (`set`, `write`, `search`, checksums, ciphers…), ported from
//! apollo-lib's `apollo_apply_bsd_code` (GPL-3.0-or-later, Damian Parrino).
//!
//! Variables keep their bytes in file order: a number is stored big-endian, which is how the
//! original's host-order values come out when written.

use super::crypt::{self, Mode};
use super::hash::{self, Crc};
use super::{glob, glob_i, offzip, scan_dec, scan_hex, scan_hex64, Session, Var};

type R = Result<(), String>;

fn fail<T>(why: &str) -> Result<T, String> {
    Err(why.to_string())
}

fn skip(s: &str) -> &str {
    s.trim_start_matches(' ')
}

fn fits(len: usize, off: i64, n: usize) -> bool {
    off >= 0 && (off as usize) <= len && n <= len - off as usize
}

/// Text after a case-insensitive prefix the caller has already matched.
fn after<'a>(s: &'a str, prefix: &str) -> &'a str {
    s.get(prefix.len()..).unwrap_or("")
}

/// Splits at the first `c`; None when it isn't there.
fn cut(s: &str, c: char) -> Option<(&str, &str)> {
    s.find(c).map(|i| (&s[..i], &s[i + 1..]))
}

fn rcut(s: &str, c: char) -> Option<(&str, &str)> {
    s.rfind(c).map(|i| (&s[..i], &s[i + 1..]))
}

/// Up to the last `)`.
fn inner(s: &str) -> &str {
    rcut(s, ')').map(|(a, _)| a).unwrap_or(s)
}

fn hex_bytes(s: &str) -> Option<Vec<u8>> {
    if s.is_empty() || s.len() % 2 != 0 {
        return None;
    }
    let nib = |c: u8| (c as char).to_digit(16).unwrap_or(0) as u8;
    Some(s.as_bytes().chunks(2).map(|p| nib(p[0]) << 4 | nib(p[1])).collect())
}

fn be_value(b: &[u8]) -> u32 {
    b.iter().fold(0u32, |v, &x| v << 8 | x as u32)
}

struct Script<'s> {
    s: &'s mut Session,
    latin1: bool,
    pointer: i64,
    range: (usize, usize),
    carry: u32,
    eof: i64,
    crc: Crc,
}

impl Script<'_> {
    fn var_index(&self, name: &str) -> Option<usize> {
        self.s.vars.iter().position(|v| v.name == name)
    }

    /// A value: "text", [variable] or hex bytes.
    fn decode(&self, s: &str) -> Option<Vec<u8>> {
        let s = skip(s);
        if glob(s, "\"*\"*") {
            let last = s.rfind('"')?;
            let t = &s[1..last];
            return Some(if self.latin1 { t.chars().map(|c| c as u32 as u8).collect() } else { t.as_bytes().to_vec() });
        }
        if glob(s, "[*]*") {
            let (name, _) = cut(&s[1..], ']')?;
            return self.s.var(name).and_then(|v| v.data.clone());
        }
        hex_bytes(s.strip_prefix("0x").unwrap_or(s))
    }

    /// A number: hex, `pointer`, `eof` or a [variable], with `+`/`-` terms.
    fn int(&self, s: &str, ptr: i64, size: usize) -> i32 {
        let mut s = skip(s);
        if let Some(r) = s.strip_prefix('+') {
            s = r;
        }
        let neg = if let Some(r) = s.strip_prefix('-') {
            s = r;
            true
        } else {
            false
        };
        let v = if s.is_empty() {
            0
        } else if glob_i(s, "pointer*") {
            (ptr as i32).wrapping_add(self.int(skip(after(s, "pointer")), ptr, size))
        } else if glob_i(s, "eof*") {
            (size as i32).wrapping_sub(1).wrapping_add(self.int(skip(after(s, "eof")), ptr, size))
        } else if glob(s, "[*]*") {
            match cut(&s[1..], ']') {
                Some((name, rest)) => match self.s.var(name) {
                    Some(Var { data: Some(d), len: 1 | 2 | 4, .. }) => (be_value(d) as i32).wrapping_add(self.int(rest, ptr, size)),
                    _ => 0,
                },
                None => 0,
            }
        } else {
            scan_hex(s).unwrap_or(0) as i32
        };
        if neg {
            v.wrapping_neg()
        } else {
            v
        }
    }

    /// "a,b)" → (a, b).
    fn start_end(&self, s: &str, size: usize) -> (i32, i32) {
        let Some((a, rest)) = cut(s, ',') else { return (0, 0) };
        let a = self.int(a, self.pointer, size);
        let b = cut(rest, ')').map(|(b, _)| self.int(b, self.pointer, size)).unwrap_or(0);
        (a, b)
    }

    fn set_var(&mut self, at: usize, data: Vec<u8>) {
        let v = &mut self.s.vars[at];
        v.len = data.len();
        v.data = Some(data);
    }
}

/// An inclusive [start, end] range clamped to the data: (start, length).
fn clamp(len: usize, s: i32, e: i32) -> (usize, usize) {
    let s = s.max(0) as usize;
    if s >= len {
        return (len, 0);
    }
    let e = if e < 0 || e as usize >= len { len - 1 } else { e as usize };
    if e < s {
        (s, 0)
    } else {
        (s, e - s + 1)
    }
}

/// The number in a `write at …` / `insert next …` address: `(decimal)` or hex.
fn address(s: &str) -> i64 {
    if glob(s, "(*)") {
        scan_dec(&s[1..]).unwrap_or(0) as i64
    } else {
        scan_hex(s).map(|v| v as i32 as i64).unwrap_or(0)
    }
}

/// `at …:` or `next …:` → (address, the rest after the colon).
fn place<'a>(s: &'a str, pointer: i64, what: &str) -> Result<(i64, &'a str), String> {
    let s = skip(s);
    let (rel, s) = if glob_i(s, "at*") {
        (false, &s[2..])
    } else if glob_i(s, "next*") {
        (true, &s[4..])
    } else {
        return Err(format!("not a valid {what} command"));
    };
    let (a, rest) = cut(skip(s), ':').ok_or("missing ':'")?;
    let off = (address(a) as i32).wrapping_add(if rel { pointer as i32 } else { 0 }) as i64;
    Ok((off, skip(rest)))
}

fn swap_words(d: &mut [u8], n: usize) {
    for w in d.chunks_exact_mut(n) {
        w.reverse();
    }
}

pub(crate) fn run(session: &mut Session, data: &mut Vec<u8>, text: &str, be: bool, latin1: bool) -> R {
    let mut sc = Script { s: session, latin1, pointer: 0, range: (0, data.len()), carry: 0, eof: 0, crc: Crc::default() };
    let lines: Vec<&str> = text.split('\n').filter(|l| !l.is_empty()).collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        i += 1;
        if glob_i(line, "carry(*)") {
            sc.carry = scan_dec(&line[6..]).unwrap_or(0).clamp(0, 4) as u32;
        } else if glob_i(line, "set *:*") {
            set(&mut sc, data, skip(&line[3..]))?;
        } else if glob_i(line, "write *:*") {
            let (off, v) = place(&line[5..], sc.pointer, "write")?;
            let bytes = if glob_i(v, "xor:*") {
                let mut x = sc.decode(skip(&v[4..])).ok_or("nothing to write")?;
                if !fits(data.len(), off, x.len()) {
                    return fail("an xor write is out of range");
                }
                for (k, b) in x.iter_mut().enumerate() {
                    *b ^= data[off as usize + k];
                }
                x
            } else if glob_i(v, "repeat(*,*)*") {
                let a = &v[7..];
                let (n, rest) = cut(a, ',').ok_or("bad repeat()")?;
                let n = sc.int(n, sc.pointer, data.len());
                let val = sc.decode(cut(rest, ')').map(|(x, _)| x).unwrap_or(rest)).ok_or("bad repeat() value")?;
                if n < 0 || val.len().saturating_mul(n as usize) > 0x1000_0000 {
                    return fail("bad repeat() count");
                }
                val.repeat(n as usize)
            } else {
                sc.decode(v).ok_or("nothing to write")?
            };
            if fits(data.len(), off, bytes.len()) {
                data[off as usize..off as usize + bytes.len()].copy_from_slice(&bytes);
            }
        } else if glob_i(line, "insert *:*") {
            let (off, v) = place(&line[6..], sc.pointer, "insert")?;
            let ins = sc.decode(v).ok_or("nothing to insert")?;
            let off = off.clamp(0, data.len() as i64) as usize;
            data.splice(off..off, ins);
        } else if glob_i(line, "delete *:*") {
            let (off, v) = place(&line[6..], sc.pointer, "delete")?;
            let mut n = if glob_i(v, "until*") {
                let find = sc.decode(skip(&v[5..])).ok_or("nothing to search for")?;
                super::sw::search(data, off, &find, 1) - off
            } else {
                let n = sc.int(v, sc.pointer, data.len()) as i64;
                if n + off > data.len() as i64 {
                    data.len() as i64 - off
                } else {
                    n
                }
            };
            let off = off.clamp(0, data.len() as i64);
            n = n.clamp(0, data.len() as i64 - off);
            data.drain(off as usize..(off + n) as usize);
        } else if glob_i(line, "search *") {
            let mut s = skip(&line[6..]);
            let mut start = 0;
            if glob(s, "next*") {
                s = skip(&s[4..]);
                start = sc.pointer;
            }
            let mut count = 1;
            if glob(s, "*:*") {
                let (a, n) = rcut(s, ':').unwrap();
                count = scan_dec(n).unwrap_or(1);
                s = a;
            }
            let find = sc.decode(s).ok_or("bad search pattern")?;
            sc.pointer = super::sw::search(data, start, &find, count as i64);
            if sc.pointer < 0 {
                return fail("the search pattern wasn't found");
            }
        } else if glob_i(line, "copy *:*:*") {
            let s = skip(&line[4..]);
            let (a, rest) = cut(s, ':').unwrap();
            let (b, c) = cut(rest, ':').unwrap();
            let n = data.len();
            let (from, to, len) = (sc.int(a, sc.pointer, n) as i64, sc.int(b, sc.pointer, n) as i64, sc.int(c, sc.pointer, n));
            if len >= 0 && fits(n, from, len as usize) && fits(n, to, len as usize) {
                data.copy_within(from as usize..from as usize + len as usize, to as usize);
            }
        } else if glob_i(line, "msgbox [*]*") {
            // Shows a value in Apollo's log; nothing to do here.
        } else if glob_i(line, "endian_swap(*)*") {
            let mode = sc.int(inner(&line[12..]), sc.pointer, data.len());
            let (a, b) = sc.range;
            match mode {
                2 | 4 | 8 => swap_words(&mut data[a..b.max(a)], mode as usize),
                _ => return fail("endian_swap() takes 2, 4 or 8"),
            }
        } else if glob_i(line, "decompress(*,*)*") {
            let s = &line[11..];
            let (a, rest) = cut(s, ',').unwrap();
            let a = skip(a);
            let (mut offset, mut count) = (0usize, 0usize);
            if let Some(n) = a.strip_prefix('#') {
                count = scan_dec(n).unwrap_or(0).max(0) as usize;
            } else if !a.starts_with('*') {
                offset = scan_hex(a).unwrap_or(0) as usize;
                count = 1;
            }
            let w = cut(rest, ')').map(|(w, _)| w).unwrap_or(rest);
            let wbits = if w.starts_with('*') { 0 } else { scan_dec(w).unwrap_or(0) };
            let found = if wbits == 0 {
                let z = offzip::scan(data, offset, offzip::WBITS_ZLIB, count);
                if z.is_empty() {
                    offzip::scan(data, offset, offzip::WBITS_DEFLATE, count)
                } else {
                    z
                }
            } else {
                offzip::scan(data, offset, wbits, count)
            };
            if found.is_empty() {
                return fail("no compressed data was found");
            }
            for f in &found {
                let name = format!("~extracted\\{:08x}.dat", f.offset);
                sc.s.vars.retain(|v| v.name != name);
                sc.s.vars.push(Var { name, len: f.data.len(), data: Some(f.data.clone()) });
            }
            sc.s.zips = found;
        } else if glob_i(line, "compress(*)*") {
            let s = &line[9..];
            let offset = if s.starts_with('*') { 0 } else { scan_hex(s).unwrap_or(0) as usize };
            if sc.s.zips.is_empty() {
                return fail("nothing was decompressed to pack again");
            }
            let mut zips = std::mem::take(&mut sc.s.zips);
            let mut ok = true;
            for f in zips.iter_mut() {
                if offset == 0 || offset == f.offset {
                    let name = format!("~extracted\\{:08x}.dat", f.offset);
                    let cur = sc.s.var(&name).and_then(|v| v.data.clone()).unwrap_or_else(|| f.data.clone());
                    if !offzip::pack(f, &cur, data) {
                        ok = false;
                        break;
                    }
                }
            }
            sc.s.zips = zips;
            if !ok {
                return fail("packing the data again failed");
            }
        } else if glob_i(line, "decrypt *") || glob_i(line, "encrypt *") {
            let mode = if line.as_bytes()[0] | 0x20 == b'e' { Mode::Encrypt } else { Mode::Decrypt };
            cipher(&mut sc, data, mode, skip(&line[7..]), be)?;
        } else if is_sw_line(line) {
            // A run of Save Wizard lines inside the script, with its own pointer.
            let mut run = String::new();
            let mut j = i - 1;
            while j < lines.len() {
                let t = skip(lines[j]);
                if t.starts_with(';') {
                    j += 1;
                    continue;
                }
                if !is_sw_line(t) {
                    break;
                }
                run.push_str(&t[..17]);
                run.push('\n');
                j += 1;
            }
            i = j;
            super::sw::run(data, &run, be)?;
        }
    }
    Ok(())
}

fn is_sw_line(l: &str) -> bool {
    let l = skip(l).as_bytes();
    l.len() == 17 && l[8] == b' ' && l.iter().enumerate().all(|(i, c)| i == 8 || c.is_ascii_hexdigit())
}

fn set(sc: &mut Script, data: &mut [u8], line: &str) -> R {
    let size = data.len();
    if glob_i(line, "pointer:*") {
        let s = &line[8..];
        let hexarg = |s: &str| if s.is_empty() { 0 } else { scan_hex(s).unwrap_or(0) as i32 as i64 };
        if glob_i(s, "eof*") || glob_i(s, "lastbyte*") {
            let n = if glob_i(s, "eof*") { 3 } else { 8 };
            sc.eof = 1;
            sc.pointer = size as i64 + hexarg(skip(&s[n..])) - 1;
        } else if glob_i(s, "pointer*") {
            sc.pointer += hexarg(skip(&s[7..]));
        } else if glob_i(s, "read(*,*)*") {
            // read(address, length): the pointer becomes the big-endian dword there.
            let t = &s[4..];
            let parsed = t.strip_prefix('(').and_then(|t| {
                let (a, b) = cut(t, ',')?;
                Some((scan_hex(a)?, scan_hex(b)?))
            });
            let mut v = 0;
            if let Some((a, _)) = parsed {
                if fits(size, a as i32 as i64, 4) {
                    let a = a as usize;
                    v = u32::from_be_bytes([data[a], data[a + 1], data[a + 2], data[a + 3]]);
                }
            }
            sc.pointer = v as i64;
        } else if glob_i(s, "[*]*") {
            sc.pointer = sc.int(s, sc.pointer, size) as i64;
        } else {
            sc.pointer = hexarg(s);
        }
        return Ok(());
    }
    if glob_i(line, "range:*,*") {
        let (a, b) = cut(&line[6..], ',').unwrap();
        let start = (sc.int(a, sc.pointer, size) as i64).clamp(0, size as i64);
        let end = ((sc.int(b, sc.pointer - sc.eof, size) as i64) + 1).min(size as i64).max(start);
        sc.range = (start as usize, end as usize);
        return Ok(());
    }
    if glob_i(line, "crc_*:*") {
        let s = &line[4..];
        let tail = |p: &str| &s[p.len()..];
        if glob_i(s, "bandwidth:*") {
            if let Some(v) = scan_dec(tail("bandwidth:")) {
                sc.crc.width = v as u32;
            }
        } else if glob_i(s, "polynomial:*") {
            if let Some(v) = scan_hex64(tail("polynomial:")) {
                sc.crc.poly = v;
            }
        } else if glob_i(s, "initial_value:[*]*") {
            sc.crc.init = sc.int(tail("initial_value:"), sc.pointer, size) as i64 as u64;
        } else if glob_i(s, "initial_value:*") {
            if let Some(v) = scan_hex64(tail("initial_value:")) {
                sc.crc.init = v;
            }
        } else if glob_i(s, "output_xor:*") {
            if let Some(v) = scan_hex64(tail("output_xor:")) {
                sc.crc.xor_out = v;
            }
        } else if glob_i(s, "reflection_input:*") {
            if let Some(v) = scan_dec(tail("reflection_input:")) {
                sc.crc.ref_in = v != 0;
            }
        } else if glob_i(s, "reflection_output:*") {
            if let Some(v) = scan_dec(tail("reflection_output:")) {
                sc.crc.ref_out = v != 0;
            }
        }
        return Ok(());
    }
    if !glob(line, "[*]:*") {
        return Ok(());
    }
    let (name, rest) = cut(&line[1..], ']').unwrap();
    let op = rest.get(1..).unwrap_or("");
    let at = match sc.var_index(name) {
        Some(at) => at,
        None => {
            sc.s.vars.push(Var { name: name.to_string(), data: None, len: 0 });
            sc.s.vars.len() - 1
        }
    };
    // The variable's value before this line (sums add to it); a value wider than four bytes
    // is dropped, as in the original.
    let old = {
        let v = &mut sc.s.vars[at];
        match &v.data {
            Some(d) if v.len == 2 || v.len == 4 || v.len <= 3 => be_value(d),
            Some(_) if v.len > 4 => {
                v.data = None;
                0
            }
            _ => 0,
        }
    };
    let (rs, re) = sc.range;
    macro_rules! region {
        ($d:expr) => {
            &$d[rs..re.max(rs)]
        };
    }
    let set32 = |sc: &mut Script, v: u32| sc.set_var(at, v.to_be_bytes().to_vec());

    let bitwise = |sc: &mut Script, s: &str, f: fn(u8, u8) -> u8| -> R {
        let mask = sc.decode(skip(s)).ok_or("bad value for a bitwise operation")?;
        let v = &sc.s.vars[at];
        match &v.data {
            Some(d) if v.len == mask.len() => {
                let r = mask.iter().zip(d).map(|(&m, &x)| f(m, x)).collect();
                sc.set_var(at, r);
                Ok(())
            }
            _ => fail("a bitwise operation's value doesn't match the variable's length"),
        }
    };
    // `name:value` → the value after the first ':' (an initial value for several hashes).
    let init = |sc: &Script, s: &str, default: u32| cut(s, ':').map(|(_, v)| sc.int(v, sc.pointer, size) as u32).unwrap_or(default);

    if glob_i(op, "xor:*") {
        return bitwise(sc, &op[4..], |a, b| a ^ b);
    }
    if glob_i(op, "and:*") {
        return bitwise(sc, &op[4..], |a, b| a & b);
    }
    if glob_i(op, "or:*") {
        return bitwise(sc, &op[3..], |a, b| a | b);
    }
    if glob_i(op, "endian_swap*") {
        let v = &sc.s.vars[at];
        return match &v.data {
            Some(d) if matches!(v.len, 2 | 4 | 8) => {
                let r = d.iter().rev().copied().collect();
                sc.set_var(at, r);
                Ok(())
            }
            _ => fail("endian_swap needs a 2-, 4- or 8-byte variable"),
        };
    }
    if glob_i(op, "[*]*") || glob_i(op, "eof*") {
        let v = sc.int(op, sc.pointer, size) as u32;
        set32(sc, v);
        return Ok(());
    }
    if glob_i(op, "crc32*") {
        sc.crc.init = 0xFFFF_FFFF;
        sc.crc.poly = hash::CRC32_POLY;
        sc.crc.xor_out = 0xFFFF_FFFF;
        if let Some((_, v)) = cut(op, ':') {
            if let Some(v) = scan_hex64(v) {
                sc.crc.init = v;
            }
        }
        let refl = !glob_i(op, "crc32big*");
        sc.crc.ref_in = refl;
        sc.crc.ref_out = refl;
        let h = hash::crc(region!(data), &sc.crc, 32) as u32;
        set32(sc, h);
        return Ok(());
    }
    if glob_i(op, "crc16*") {
        sc.crc = Crc { width: sc.crc.width, poly: 0x1021, init: 0, xor_out: 0, ref_in: false, ref_out: false };
        let h = hash::crc(region!(data), &sc.crc, 16) as u16;
        sc.set_var(at, h.to_be_bytes().to_vec());
        return Ok(());
    }
    if glob_i(op, "crc64*") {
        let ecma = glob_i(op, "crc64_ecma*");
        sc.crc = if ecma {
            Crc { width: sc.crc.width, poly: 0x42F0_E1EB_A9EA_3693, init: 0, xor_out: 0, ref_in: false, ref_out: false }
        } else {
            Crc { width: sc.crc.width, poly: 0x1B, init: u64::MAX, xor_out: u64::MAX, ref_in: true, ref_out: true }
        };
        let h = hash::crc(region!(data), &sc.crc, 64);
        sc.set_var(at, h.to_be_bytes().to_vec());
        return Ok(());
    }
    if glob_i(op, "crc*") {
        let r = region!(data);
        let bytes = match sc.crc.width {
            16 => (hash::crc(&r, &sc.crc, 16) as u16).to_be_bytes().to_vec(),
            64 => hash::crc(&r, &sc.crc, 64).to_be_bytes().to_vec(),
            _ => (hash::crc(&r, &sc.crc, 32) as u32).to_be_bytes().to_vec(),
        };
        sc.set_var(at, bytes);
        return Ok(());
    }
    use md5::Digest;
    let digest: Option<Vec<u8>> = if glob_i(op, "md5_xor*") {
        Some(hash::md5_xor(region!(data)).to_be_bytes().to_vec())
    } else if glob_i(op, "md5*") {
        Some(md5::Md5::digest(region!(data)).to_vec())
    } else if glob_i(op, "sha224*") {
        Some(sha2::Sha224::digest(region!(data)).to_vec())
    } else if glob_i(op, "sha1_xor64*") {
        Some(hash::sha1_xor64(region!(data)).to_be_bytes().to_vec())
    } else if glob_i(op, "sha1*") {
        Some(sha1::Sha1::digest(region!(data)).to_vec())
    } else if glob_i(op, "sha256*") {
        Some(sha2::Sha256::digest(region!(data)).to_vec())
    } else if glob_i(op, "sha384*") {
        Some(sha2::Sha384::digest(region!(data)).to_vec())
    } else if glob_i(op, "sha512*") {
        Some(sha2::Sha512::digest(region!(data)).to_vec())
    } else if glob_i(op, "adler32*") {
        Some(hash::adler32(init(sc, op, 1), region!(data)).to_be_bytes().to_vec())
    } else if glob_i(op, "adler16*") {
        Some(hash::adler16(region!(data)).to_be_bytes().to_vec())
    } else if glob_i(op, "fletcher16*") {
        Some(hash::fletcher16(region!(data)).to_be_bytes().to_vec())
    } else if glob_i(op, "fletcher32*") {
        Some(hash::fletcher32(region!(data)).to_be_bytes().to_vec())
    } else if glob_i(op, "murmur3_32*") {
        Some(hash::murmur3_32(region!(data), init(sc, op, 0)).to_be_bytes().to_vec())
    } else if glob_i(op, "jhash*") {
        Some(hash::jhash(region!(data), init(sc, op, 0)).to_be_bytes().to_vec())
    } else if glob_i(op, "jenkins_oaat*") {
        Some(hash::jenkins_oaat(region!(data), init(sc, op, 0)).to_be_bytes().to_vec())
    } else if glob_i(op, "hmac_sha1(*)*") {
        use hmac::Mac;
        let key = sc.decode(inner(&op[10..])).ok_or("bad HMAC key")?;
        let mut m = hmac::Hmac::<sha1::Sha1>::new_from_slice(&key).map_err(|e| e.to_string())?;
        m.update(region!(data));
        Some(m.finalize().into_bytes().to_vec())
    } else if glob_i(op, "force_crc32:*") {
        let want = sc.int(cut(op, ':').unwrap().1, sc.pointer, size) as u32;
        Some(hash::force_crc32(region!(data), sc.pointer as u32, want).to_be_bytes().to_vec())
    } else if glob_i(op, "eachecksum*") {
        Some(hash::mc02(region!(data)).to_be_bytes().to_vec())
    } else if glob_i(op, "ffx_checksum*") {
        Some(hash::ffx(region!(data)).to_le_bytes().to_vec())
    } else if glob_i(op, "ff13_checksum*") {
        Some(hash::ff13(region!(data)).to_le_bytes().to_vec())
    } else if glob_i(op, "castlevania_checksum*") {
        Some(hash::castlevania(region!(data)).to_le_bytes().to_vec())
    } else if glob_i(op, "deadrising_checksum*") {
        hash::deadrising(&mut data[rs..re.max(rs)]);
        None
    } else if glob_i(op, "dbzxv2_checksum*") {
        Some(hash::dbzxv2(data).to_be_bytes().to_vec())
    } else if glob_i(op, "rockstar_checksum*") {
        Some(rockstar(data, rs, re).to_be_bytes().to_vec())
    } else if glob_i(op, "lookup3_little2(*,*)*") {
        let (a, b) = sc.start_end(&op[16..], size);
        let (c, b) = hash::lookup3_little2(region!(data), a as u32, b as u32);
        Some((b as u64 + ((c as u64) << 32)).to_be_bytes().to_vec())
    } else if glob_i(op, "kh25_checksum*") {
        Some(hash::kh25(region!(data)).to_le_bytes().to_vec())
    } else if glob_i(op, "khcom_checksum*") {
        Some(hash::khcom(region!(data)).to_be_bytes().to_vec())
    } else if glob_i(op, "mgs2_checksum*") {
        Some(hash::mgs2(region!(data)).to_be_bytes().to_vec())
    } else if glob_i(op, "mgspw_checksum*") {
        Some(hash::mgspw(region!(data)).to_be_bytes().to_vec())
    } else if glob_i(op, "sw4_checksum*") {
        Some(hash::sw4(region!(data)).iter().flat_map(|w| w.to_be_bytes()).collect())
    } else if glob_i(op, "toz_checksum*") {
        Some(hash::toz(region!(data)).to_vec())
    } else if glob_i(op, "tiara2_checksum*") {
        Some(hash::tiara2(region!(data)).to_be_bytes().to_vec())
    } else if glob_i(op, "checksum32*") {
        Some(hash::checksum32(region!(data)).to_be_bytes().to_vec())
    } else if glob_i(op, "sdbm*") {
        Some(hash::sdbm(region!(data), init(sc, op, 0)).to_be_bytes().to_vec())
    } else if glob_i(op, "djb2*") {
        Some(hash::djb2(region!(data)).to_be_bytes().to_vec())
    } else if glob_i(op, "fnv1*") {
        Some(hash::fnv1(region!(data), init(sc, op, 0x811c_9dc5)).to_be_bytes().to_vec())
    } else {
        sum_or_slice(sc, data, op, old, at)?
    };
    if let Some(d) = digest {
        sc.set_var(at, d);
    }
    Ok(())
}

/// The sums over a range (`add`, `wadd`…), `xor`, `read`, `left`/`right`/`mid` and plain values.
fn sum_or_slice(sc: &mut Script, data: &[u8], op: &str, old: u32, at: usize) -> Result<Option<Vec<u8>>, String> {
    let size = data.len();
    let span = |sc: &Script, s: &str| {
        let (a, b) = sc.start_end(s, size);
        let (s, n) = clamp(size, a, b);
        data[s..s + n].to_vec()
    };
    let carry = |mut v: u32, c: u32| {
        while c > 0 && v > 0xFFFF {
            v = (v & 0xFFFF).wrapping_add(((v & 0xFFFF_0000) as u64 >> (8 * c)) as u32);
        }
        v.to_be_bytes()[c as usize..].to_vec()
    };
    let r = if glob_i(op, "qwadd(*,*)*") {
        old.wrapping_add(hash::qwadd(&span(sc, &op[6..]))).to_be_bytes().to_vec()
    } else if glob_i(op, "dwadd(*,*)*") {
        old.wrapping_add(hash::dwadd(&span(sc, &op[6..]), false)).to_be_bytes().to_vec()
    } else if glob_i(op, "wadd_le(*,*)*") {
        old.wrapping_add(hash::wadd(&span(sc, &op[8..]), true)).to_be_bytes().to_vec()
    } else if glob_i(op, "dwadd_le(*,*)*") {
        old.wrapping_add(hash::dwadd(&span(sc, &op[9..]), true)).to_be_bytes().to_vec()
    } else if glob_i(op, "wadd(*,*)*") {
        carry(old.wrapping_add(hash::wadd(&span(sc, &op[5..]), false)), sc.carry)
    } else if glob_i(op, "add(*,*)*") {
        carry(old.wrapping_add(hash::add(&span(sc, &op[4..]))), sc.carry)
    } else if glob_i(op, "wsub(*,*)*") {
        old.wrapping_add(hash::wsub(&span(sc, &op[5..]))).to_be_bytes().to_vec()
    } else if glob_i(op, "xor(*,*,*)*") {
        let s = &op[4..];
        let (a, rest) = cut(s, ',').unwrap();
        let (b, rest) = cut(rest, ',').unwrap();
        let c = cut(rest, ')').map(|(c, _)| c).unwrap_or(rest);
        let p = sc.pointer;
        let (s, e, step) = (sc.int(a, p, size), sc.int(b, p, size), sc.int(c, p, size).clamp(1, 4) as usize);
        let s = s.clamp(0, size as i32) as usize;
        let e = e.clamp(0, size as i32) as usize;
        let mut x = [0u8; 4];
        let mut at = s;
        while at < e && at + step <= size {
            for j in 0..step {
                x[j] ^= data[at + j];
            }
            at += step;
        }
        x.to_vec()
    } else if glob_i(op, "read(*,*)*") {
        let (a, n) = sc.start_end(&op[5..], size);
        if n < 0 || !fits(size, a as i64, n as usize) {
            return fail("read() is out of range");
        }
        data[a as usize..a as usize + n as usize].to_vec()
    } else if glob_i(op, "right(*,*)*") || glob_i(op, "left(*,*)*") {
        let left = glob_i(op, "left(*,*)*");
        let (v, n) = sc.start_end(&op[if left { 5 } else { 6 }..], size);
        if !(0..=4).contains(&n) {
            return fail("left()/right() take up to 4 bytes");
        }
        let b = (v as u32).to_be_bytes();
        if left {
            b[..n as usize].to_vec()
        } else {
            b[4 - n as usize..].to_vec()
        }
    } else if glob_i(op, "mid(*,*,*)*") {
        let s = &op[4..];
        let (a, rest) = cut(s, ',').unwrap();
        let (b, rest) = cut(rest, ',').unwrap();
        let c = cut(rest, ')').map(|(c, _)| c).unwrap_or(rest);
        let v = sc.decode(a);
        let (st, n) = (sc.int(b, sc.pointer, size), sc.int(c, sc.pointer, size));
        match v {
            Some(v) if st >= 0 && n >= 0 && st as usize <= v.len() && n as usize <= v.len() - st as usize => v[st as usize..(st + n) as usize].to_vec(),
            _ => return fail("bad mid() arguments"),
        }
    } else if glob_i(op, "host_*") {
        // The console's account ID, PSID and the like: there is no console here.
        return Ok(None);
    } else if glob_i(op, "0x*") && op.len() <= 10 {
        scan_hex(op).unwrap_or(0).to_be_bytes().to_vec()
    } else {
        match sc.decode(op) {
            Some(v) => v,
            None => {
                sc.s.vars[at].data = None;
                sc.s.vars[at].len = 0;
                return Ok(None);
            }
        }
    };
    Ok(Some(r))
}


/// Rockstar's "CHKS" blocks: each gets a fresh one-at-a-time hash. Returns the variable's value.
fn rockstar(d: &mut [u8], rs: usize, re: usize) -> u32 {
    let len = re.saturating_sub(rs);
    let lim = d.len().min(len);
    let find = |d: &[u8], from: i64| super::sw::search(&d[..lim], from, b"CHKS\0", 1);
    let mut chks: u32 = 0;
    let mut off = find(d, rs as i64);
    while off > 0 {
        let o = off as usize;
        if !fits(d.len(), off, 0x10) {
            break;
        }
        chks = u32::from_be_bytes(d[o + 4..o + 8].try_into().unwrap());
        let chks_len = u32::from_be_bytes(d[o + 8..o + 12].try_into().unwrap());
        let blk = off - chks_len as i64 + chks as i64;
        if blk < 0 || blk > d.len() as i64 || !fits(d.len(), blk, chks_len as usize) {
            off = find(d, off + 1);
            continue;
        }
        d[o + 8..o + 16].fill(0);
        let h = hash::jenkins_oaat(&d[blk as usize..blk as usize + chks_len as usize], 0x3FAC_7125);
        d[o + 12..o + 16].copy_from_slice(&h.to_be_bytes());
        d[o + 8..o + 12].copy_from_slice(&chks_len.to_be_bytes());
        chks = h.swap_bytes();
        off = find(d, off + 1);
    }
    chks
}

fn cipher(sc: &mut Script, data: &mut [u8], mode: Mode, s: &str, be: bool) -> R {
    let (rs, re) = sc.range;
    let size = data.len();
    let key = |sc: &Script, t: &str| sc.decode(inner(t)).ok_or_else(|| "bad key".to_string());
    let key_iv = |sc: &Script, t: &str| -> Result<(Vec<u8>, Vec<u8>), String> {
        let (k, rest) = rcut(t, ',').ok_or("bad key")?;
        let k = sc.decode(k).ok_or("bad key")?;
        let iv = sc.decode(inner(rest)).ok_or("bad IV")?;
        Ok((k, iv))
    };
    let num = |sc: &Script, t: &str| sc.int(inner(t), sc.pointer, size);
    let d = &mut data[rs..re.max(rs)];
    if glob_i(s, "diablo3*") {
        crypt::diablo3(mode, d);
    } else if glob_i(s, "dw8xl*") {
        crypt::dw8xl(d);
    } else if glob_i(s, "silent_hill3*") {
        crypt::silent_hill3(mode, d);
    } else if glob_i(s, "nfs_undercover*") {
        crypt::nfs_undercover(mode, d);
    } else if glob_i(s, "ffxiii(*,*)*") {
        let t = &s[7..];
        let (g, rest) = rcut(t, ',').ok_or("bad ffxiii() arguments")?;
        let game = sc.int(g, sc.pointer, size);
        let k = sc.decode(inner(rest)).ok_or("bad ffxiii() key")?;
        crypt::final_fantasy13(mode, game as u32, d, &k);
    } else if glob_i(s, "rgg_studio(*)*") {
        crypt::rgg_studio(d, &key(sc, &s[11..])?);
    } else if glob_i(s, "borderlands3(*)*") {
        crypt::borderlands3(mode, d, num(sc, &s[13..]) != 0);
    } else if glob_i(s, "monster_hunter(*)*") {
        crypt::monster_hunter(mode, d, num(sc, &s[15..]));
    } else if glob_i(s, "mgs5_tpp(*)*") {
        crypt::mgs5_tpp(d, num(sc, &s[9..]) as u32, be);
    } else if glob_i(s, "mgs_pw(*)*") {
        crypt::mgs_pw(mode, d, num(sc, &s[7..]));
    } else if glob_i(s, "mgs_base64*") {
        crypt::mgs_base64(mode, d);
    } else if glob_i(s, "mgs(*)*") {
        crypt::mgs(mode, d, &key(sc, &s[4..])?);
    } else if glob_i(s, "aes_ecb(*)*") {
        crypt::aes_ecb(mode, d, &key(sc, &s[8..])?);
    } else if glob_i(s, "aes_cbc(*,*)*") {
        let (k, iv) = key_iv(sc, &s[8..])?;
        crypt::aes_cbc(mode, d, &k, &iv);
    } else if glob_i(s, "aes_ctr(*,*)*") {
        let (k, iv) = key_iv(sc, &s[8..])?;
        crypt::aes_ctr(d, &k, &iv);
    } else if glob_i(s, "camellia_ecb(*)*") {
        crypt::camellia_ecb(mode, d, &key(sc, &s[13..])?);
    } else if glob_i(s, "des3_ecb(*)*") {
        crypt::des3_ecb(mode, d, &key(sc, &s[9..])?);
    } else if glob_i(s, "des3_cbc(*,*)*") {
        let (k, iv) = key_iv(sc, &s[9..])?;
        crypt::des3_cbc(mode, d, &k, &iv);
    } else if glob_i(s, "blowfish_ecb(*)*") {
        crypt::blowfish_ecb(mode, d, &key(sc, &s[13..])?);
    } else if glob_i(s, "blowfish_cbc(*)*") {
        let (k, iv) = key_iv(sc, &s[13..])?;
        crypt::blowfish_cbc(mode, d, &k, &iv);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn go(d: &mut Vec<u8>, script: &str) -> R {
        let mut s = Session::new(false);
        run(&mut s, d, script, false, false)
    }

    #[test]
    fn scripts() {
        let mut d = vec![0u8; 64];
        go(&mut d, "write at 0x10:\"Difficulty\"\nwrite at (2):0A0B\n").unwrap();
        assert_eq!(&d[0x10..0x1A], b"Difficulty");
        assert_eq!(&d[2..4], &[0x0A, 0x0B]);
        go(&mut d, "search \"Diff\"\nwrite next 4:\"X\"\n").unwrap();
        assert_eq!(d[0x14], b'X');
        assert!(go(&mut d, "search \"nope\"\nwrite at 0:FF\n").is_err());
        // Read, OR, write back.
        go(&mut d, "set [b]:read(0x02,2)\nset [b]:or:F000\nwrite at 0x20:[b]\n").unwrap();
        assert_eq!(&d[0x20..0x22], &[0xFA, 0x0B]);
        // A byte sum kept to two bytes with carry.
        go(&mut d, "set range:0x10,0x19\nset [s]:0\ncarry(2)\nset [s]:add(0x10,0x19)\nwrite at 0x30:[s]\n").unwrap();
        let sum: u32 = d[0x10..0x1A].iter().map(|&b| b as u32).sum();
        assert_eq!(&d[0x30..0x32], &(sum as u16).to_be_bytes());
        // CRC-32 over a range, written big-endian; endian_swap for little-endian.
        go(&mut d, "set range:0x10,0x19\nset [c]:crc32\nset [c]:endian_swap\nwrite at 0x38:[c]\n").unwrap();
        let c = hash::crc(&d[0x10..0x1A], &Crc { width: 32, poly: hash::CRC32_POLY, init: !0u32 as u64, xor_out: !0u32 as u64, ref_in: true, ref_out: true }, 32) as u32;
        assert_eq!(&d[0x38..0x3C], &c.to_le_bytes());
        // Insert and delete change the size.
        go(&mut d, "insert at 0:AABB\n").unwrap();
        assert_eq!((d.len(), d[0], d[2]), (66, 0xAA, 0));
        go(&mut d, "delete at 0:2\n").unwrap();
        assert_eq!(d.len(), 64);
        // Save Wizard lines inside a script.
        go(&mut d, "write at 0:01\n00000001 00000002\n").unwrap();
        assert_eq!(&d[0..2], &[1, 2]);
        // left/right/mid slices.
        go(&mut d, "set [v]:right(0x11223344,3)\nwrite at 0:[v]\nset [w]:mid(\"abcdef\",2,3)\nwrite at 4:[w]\n").unwrap();
        assert_eq!(&d[0..3], &[0x22, 0x33, 0x44]);
        assert_eq!(&d[4..7], b"cde");
    }

    #[test]
    fn compressed_blocks() {
        use std::io::Write;
        let inner: Vec<u8> = (0..2000u32).map(|i| (i.wrapping_mul(2654435761) >> 13) as u8).collect();
        let mut e = flate2::write::ZlibEncoder::new(vec![], flate2::Compression::default());
        e.write_all(&inner).unwrap();
        let mut d = vec![0x11u8; 8];
        d.extend(e.finish().unwrap());
        let mut s = Session::new(false);
        run(&mut s, &mut d, "decompress(*,15)\n", false, false).unwrap();
        let code = super::super::parse(":~extracted\\00000008.dat\n[x]\nwrite at 0:FF\n", false);
        s.apply(&mut d, &code[0], &[]).unwrap();
        run(&mut s, &mut d, "compress(*)\n", false, false).unwrap();
        let f = offzip::scan(&d, 0, offzip::WBITS_ZLIB, 1);
        assert_eq!(f[0].data[0], 0xFF);
        assert_eq!(&f[0].data[1..], &inner[1..]);
    }
}

