//! Save Wizard / Game Genie codes (`XXXXXXXX YYYYYYYY` lines), ported from apollo-lib's
//! `apollo_apply_sw_code` (GPL-3.0-or-later, Damian Parrino). Out-of-range writes are
//! skipped, as there.

use super::hex_at;

/// Byte width of a value from the size nibble's low two bits (0→1, 1→2, 2→4, 3→8).
fn width(t: u8) -> usize {
    let n = (t as char).to_digit(16).unwrap_or(0);
    1 << (n & 3)
}

fn fits(len: usize, off: i64, n: usize) -> bool {
    off >= 0 && (off as usize) <= len && n <= len - off as usize
}

fn read(d: &[u8], at: usize, n: usize, be: bool) -> u64 {
    let s = &d[at..at + n];
    if be {
        s.iter().fold(0u64, |v, &b| v << 8 | b as u64)
    } else {
        s.iter().rev().fold(0u64, |v, &b| v << 8 | b as u64)
    }
}

fn write(d: &mut [u8], at: usize, n: usize, v: u64, be: bool) {
    for i in 0..n {
        let byte = (v >> (8 * i)) as u8;
        d[at + if be { n - 1 - i } else { i }] = byte;
    }
}

/// Up to 4·k bytes of a search or write pattern, read from code lines (both halves).
fn pattern<'a>(first: u32, len: usize, lines: &mut impl Iterator<Item = &'a str>) -> Option<Vec<u8>> {
    let mut p = first.to_be_bytes().to_vec();
    let mut i = 4;
    while i < len {
        let l = lines.next().filter(|l| l.len() >= 17)?;
        p.extend(hex_at(l, 0, 8).to_be_bytes());
        if i + 4 < len {
            p.extend(hex_at(l, 9, 8).to_be_bytes());
        }
        i += 8;
    }
    p.truncate(len);
    Some(p)
}

pub(crate) fn search(d: &[u8], start: i64, find: &[u8], count: i64) -> i64 {
    if d.len() < find.len() || start < 0 || find.is_empty() {
        return -1;
    }
    let mut k = 1;
    for i in start as usize..=d.len() - find.len() {
        if &d[i..i + find.len()] == find {
            if k == count {
                return i as i64;
            }
            k += 1;
        }
    }
    -1
}

fn rsearch(d: &[u8], start: i64, find: &[u8], count: i64) -> i64 {
    if d.len() < find.len() || find.is_empty() {
        return -1;
    }
    let last = (d.len() - find.len()) as i64;
    let mut k = 1;
    let mut i = start;
    while i >= 0 {
        if i <= last && &d[i as usize..i as usize + find.len()] == find {
            if k == count {
                return i;
            }
            k += 1;
        }
        i -= 1;
    }
    -1
}

/// Runs a Save Wizard code over `d` in place.
pub(crate) fn run(d: &mut [u8], text: &str, be: bool) -> Result<(), String> {
    let lines: Vec<&str> = text.split('\n').filter(|l| !l.is_empty()).collect();
    let mut it = lines.into_iter();
    let mut pointer: i64 = 0;
    let mut end_pointer: i64 = 0;
    let mut ptr_value: u32 = 0;
    let len = d.len();
    let mut next = it.next();
    while let Some(line) = next {
        let b = line.as_bytes();
        if b.len() < 17 {
            next = it.next();
            continue;
        }
        let t = b[1];
        let val = hex_at(line, 9, 8);
        let ptr_if = |on: &[u8]| if on.contains(&t) { pointer } else { 0 };
        match b[0] {
            b'0' | b'1' | b'2' => {
                let n = 1usize << (b[0] - b'0');
                let off = hex_at(line, 2, 6) as i64 + if t == b'8' { pointer } else { 0 };
                if fits(len, off, n) {
                    write(d, off as usize, n, val as u64, be);
                }
            }
            b'3' => {
                let off = hex_at(line, 2, 6) as i64 + ptr_if(b"89ABCDEF");
                let n = width(t);
                if fits(len, off, n) {
                    let at = off as usize;
                    let cur = read(d, at, n, be);
                    let sub = matches!(t, b'4'..=b'7' | b'C'..=b'F');
                    let amount = match n {
                        1 => val as u64 & 0xFF,
                        2 => val as u64 & 0xFFFF,
                        _ => val as u64,
                    };
                    let mask = if n == 8 { u64::MAX } else { (1u64 << (8 * n)) - 1 };
                    let v = if sub { cur.wrapping_sub(amount) } else { cur.wrapping_add(amount) } & mask;
                    write(d, at, n, v, be);
                }
            }
            b'4' => {
                let off = hex_at(line, 2, 6) as i64 + ptr_if(b"89ACDE");
                let Some(l2) = it.next().filter(|l| l.len() >= 17) else {
                    return Err("a multi-write code is cut short".into());
                };
                let n = if b"456CDE".contains(&t) { hex_at(l2, 0, 4) } else { hex_at(l2, 1, 3) };
                let step = hex_at(l2, 4, 4) as i64;
                let inc = hex_at(l2, 9, 8);
                let w = width(t);
                let mut v = val;
                for i in 0..n as i64 {
                    let at = off + step * i;
                    if w <= 4 && fits(len, at, w) {
                        write(d, at as usize, w, v as u64, be);
                    }
                    v = v.wrapping_add(inc);
                }
            }
            b'5' => {
                let src = hex_at(line, 2, 6) as i64 + if t == b'8' { pointer } else { 0 };
                let Some(l2) = it.next().filter(|l| l.len() >= 17) else {
                    return Err("a copy code is cut short".into());
                };
                let dst = hex_at(l2, 2, 6) as i64 + if l2.as_bytes()[1] == b'8' { pointer } else { 0 };
                let n = val as usize;
                if fits(len, src, n) && fits(len, dst, n) {
                    d.copy_within(src as usize..src as usize + n, dst as usize);
                }
            }
            b'6' => {
                let (w, x, y, z) = (b[2], b[3], b[5], b[7]);
                let off = if b"89A".contains(&t) { pointer } else { 0 };
                let n = width(t);
                match w {
                    b'0' => {
                        let v = val.wrapping_add(if x == b'1' { ptr_value } else { 0 });
                        if y == b'1' {
                            pointer = v as i64;
                        }
                        let at = v as i64 + off;
                        if n <= 4 && fits(len, at, n) {
                            ptr_value = read(d, at as usize, n, be) as u32;
                        }
                    }
                    b'1' => {
                        ptr_value = match x {
                            b'0' => ptr_value.wrapping_add(val),
                            b'1' => ptr_value.wrapping_sub(val),
                            b'2' => ptr_value.wrapping_mul(val),
                            _ => ptr_value,
                        };
                        if z == b'1' {
                            ptr_value = ptr_value.wrapping_add(pointer as u32);
                        }
                        pointer = ptr_value as i64;
                    }
                    b'2' => {
                        pointer = match x {
                            b'0' => pointer.wrapping_add(val as i64),
                            b'1' => pointer.wrapping_sub(val as i64),
                            b'2' => pointer.wrapping_mul(val as i64),
                            _ => pointer,
                        };
                        if y == b'1' {
                            ptr_value = pointer as u32;
                        }
                    }
                    b'4' => {
                        if n <= 4 && fits(len, pointer, n) {
                            write(d, pointer as usize, n, val as u64, be);
                        }
                    }
                    _ => {}
                }
            }
            b'7' => {
                let off = hex_at(line, 2, 6) as i64 + ptr_if(b"89ACDE");
                let n = width(t);
                if fits(len, off, n) && n <= 4 {
                    let at = off as usize;
                    let mask = if n == 4 { 0xFFFF_FFFF } else { (1u64 << (8 * n)) - 1 };
                    let v = val as u64 & mask;
                    let cur = read(d, at, n, be);
                    let no_less = matches!(t, b'0'..=b'2' | b'8'..=b'A');
                    let new = if no_less { cur.max(v) } else { cur.min(v) };
                    write(d, at, n, new, be);
                }
            }
            b'8' | b'B' => {
                let cnt = hex_at(line, 2, 2).max(1) as i64;
                let plen = hex_at(line, 4, 4) as usize;
                if plen == 0 {
                    return Err("a search code has an empty pattern".into());
                }
                let Some(find) = pattern(val, plen, &mut it) else {
                    return Err("a search pattern is cut short".into());
                };
                let found = if b[0] == b'8' {
                    search(d, if t == b'8' { pointer } else { 0 }, &find, cnt)
                } else {
                    if end_pointer == 0 {
                        end_pointer = len as i64 - 1;
                    }
                    rsearch(d, if t == b'8' { pointer } else { end_pointer }, &find, cnt)
                };
                if found < 0 {
                    pointer = 0;
                    next = skip_to_search(&mut it);
                    continue;
                }
                pointer = found;
            }
            b'9' => {
                let off = val as i64;
                match t {
                    b'0' | b'1' => {
                        if fits(len, off, 4) {
                            pointer = read(d, off as usize, 4, t == b'0') as i64;
                        }
                    }
                    b'2' => pointer = pointer.wrapping_add(off),
                    b'3' => pointer = pointer.wrapping_sub(off),
                    b'4' => pointer = len as i64 - off,
                    b'5' => pointer = off,
                    b'D' => end_pointer = off,
                    b'E' => end_pointer = pointer + off,
                    _ => {}
                }
            }
            b'A' => {
                let off = hex_at(line, 2, 6) as i64 + if t == b'8' { pointer } else { 0 };
                let size = val as usize;
                let mut buf = vec![0u8; (size + 3) & !3];
                let mut i = 0;
                while i < size {
                    let Some(l) = it.next().filter(|l| l.len() >= 17) else {
                        return Err("a multi-line write is cut short".into());
                    };
                    buf[i..i + 4].copy_from_slice(&hex_at(l, 0, 8).to_be_bytes());
                    if i + 4 < size {
                        buf[i + 4..i + 8].copy_from_slice(&hex_at(l, 9, 8).to_be_bytes());
                    }
                    i += 8;
                }
                if fits(len, off, size) {
                    d[off as usize..off as usize + size].copy_from_slice(&buf[..size]);
                }
            }
            b'C' => {
                let cnt = hex_at(line, 2, 2).max(1) as i64;
                let plen = hex_at(line, 4, 4) as usize;
                let addr = (val as i64 + if t == b'8' || t == b'C' { pointer } else { 0 }) as u32 as i64;
                if !fits(len, addr, plen) {
                    pointer = 0;
                    next = skip_to_search(&mut it);
                    continue;
                }
                let a = addr as usize;
                let find = d[a..a + plen].to_vec();
                let found = if t == b'4' || t == b'C' {
                    search(&d[..a + plen], 0, &find, cnt)
                } else {
                    search(d, (a + plen) as i64, &find, cnt)
                };
                if found < 0 {
                    pointer = 0;
                    next = skip_to_search(&mut it);
                    continue;
                }
                pointer = found;
            }
            b'D' => {
                let off = hex_at(line, 2, 6) as i64 + if t == b'8' { pointer } else { 0 };
                let skip = hex_at(line, 9, 2);
                let mut v = hex_at(line, 13, 4) as i32;
                let (bit, op) = (b[11], b[12]);
                let mut src: u16 = 0;
                if fits(len, off, if bit == b'1' { 1 } else { 2 }) {
                    let o = off as usize;
                    match bit {
                        b'0' => src = (d[o] as u16) << 8 | d[o + 1] as u16,
                        b'1' => {
                            v &= 0xFF;
                            src = d[o] as u16;
                        }
                        b'2' => src = (d[o + 1] as u16) << 8 | d[o] as u16,
                        _ => {}
                    }
                }
                let s = src as i32;
                let pass = match op {
                    b'0' => s == v,
                    b'1' => s != v,
                    b'2' => s > v,
                    b'3' => s < v,
                    _ => true,
                };
                if !pass {
                    for _ in 0..skip {
                        it.next();
                    }
                }
            }
            _ => {}
        }
        next = it.next();
    }
    Ok(())
}

/// After a search misses: skip to the next search that doesn't depend on the pointer.
fn skip_to_search<'a>(it: &mut impl Iterator<Item = &'a str>) -> Option<&'a str> {
    it.find(|l| {
        let b = l.as_bytes();
        !b.is_empty() && matches!(b[0], b'8' | b'B' | b'C') && b.get(1) != Some(&b'8')
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn go(d: &mut Vec<u8>, code: &str, be: bool) {
        run(d, code, be).unwrap();
    }

    #[test]
    fn opcodes() {
        let mut d = vec![0u8; 32];
        go(&mut d, "20000004 11223344\n10000010 0000ABCD\n", false);
        assert_eq!(&d[4..8], &[0x44, 0x33, 0x22, 0x11]);
        assert_eq!(&d[0x10..0x12], &[0xCD, 0xAB]);
        go(&mut d, "20000004 11223344\n", true);
        assert_eq!(&d[4..8], &[0x11, 0x22, 0x33, 0x44]);
        // Search then write relative to the find.
        go(&mut d, "80010004 11223344\n08000004 000000EE\n", true);
        assert_eq!(d[8], 0xEE);
        // A missed search skips the codes that use its pointer.
        go(&mut d, "80010004 DEADBEEF\n08000000 00000077\n00000001 00000066\n", true);
        assert_eq!((d[0], d[1]), (0, 0));
        // Add, multi-write, conditional.
        go(&mut d, "30000010 00000001\n", false);
        assert_eq!(&d[0x10..0x11], &[0xCE]);
        go(&mut d, "40000018 00000001\n40030001 00000001\n", false);
        assert_eq!(&d[0x18..0x1B], &[1, 2, 3]);
        go(&mut d, "70000018 00000005\n74000019 00000001\n", false);
        assert_eq!(&d[0x18..0x1A], &[5, 1]);
        // Byte test: skip one line when the test fails.
        go(&mut d, "D0000018 01100009\n0000001F 00000099\n0000001E 00000042\n", false);
        assert_eq!((d[0x1E], d[0x1F]), (0x42, 0));
        // Bulk write.
        go(&mut d, "A0000000 00000006\n01020304 05060000\n", false);
        assert_eq!(&d[0..7], &[1, 2, 3, 4, 5, 6, 0x33]);
    }
}
