//! Checksum finder for saves the editor doesn't know: tries the usual algorithms (CRC-32,
//! CRC-32C, Adler-32, Fletcher-32, FNV-1a, CRC-16, additive sums, MD5, SHA-1, SHA-256) with the
//! value stored near the start or the end of the file, over the bytes after it, the bytes before
//! it, or the whole file with the value's own bytes taken as zero. A hit becomes a fix-up that
//! writes the value again every time the file is saved.

use md5::Md5;
use sha1::Sha1;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Algo {
    Crc32,
    Crc32c,
    Adler32,
    Fletcher32,
    Fnv1a32,
    Sum32,
    Words32,
    Crc16Ccitt,
    Crc16Arc,
    Sum16,
    Sum8,
    Xor8,
    Md5,
    Sha1,
    Sha256,
}

pub const ALGOS: [Algo; 15] = [
    Algo::Crc32,
    Algo::Crc32c,
    Algo::Adler32,
    Algo::Fletcher32,
    Algo::Fnv1a32,
    Algo::Sum32,
    Algo::Words32,
    Algo::Crc16Ccitt,
    Algo::Crc16Arc,
    Algo::Sum16,
    Algo::Sum8,
    Algo::Xor8,
    Algo::Md5,
    Algo::Sha1,
    Algo::Sha256,
];

impl Algo {
    pub fn name(self) -> &'static str {
        match self {
            Algo::Crc32 => "CRC-32",
            Algo::Crc32c => "CRC-32C",
            Algo::Adler32 => "Adler-32",
            Algo::Fletcher32 => "Fletcher-32",
            Algo::Fnv1a32 => "FNV-1a (32-bit)",
            Algo::Sum32 => "sum of bytes (32-bit)",
            Algo::Words32 => "sum of 32-bit words",
            Algo::Crc16Ccitt => "CRC-16/CCITT",
            Algo::Crc16Arc => "CRC-16/ARC",
            Algo::Sum16 => "sum of bytes (16-bit)",
            Algo::Sum8 => "sum of bytes (8-bit)",
            Algo::Xor8 => "XOR of bytes",
            Algo::Md5 => "MD5",
            Algo::Sha1 => "SHA-1",
            Algo::Sha256 => "SHA-256",
        }
    }

    pub fn width(self) -> usize {
        match self {
            Algo::Crc16Ccitt | Algo::Crc16Arc | Algo::Sum16 => 2,
            Algo::Sum8 | Algo::Xor8 => 1,
            Algo::Md5 => 16,
            Algo::Sha1 => 20,
            Algo::Sha256 => 32,
            _ => 4,
        }
    }

    fn is_hash(self) -> bool {
        self.width() > 4
    }

    /// The checksum of `d` as a number (hashes: not used).
    fn value(self, d: &[u8]) -> u64 {
        match self {
            Algo::Crc32 => {
                let mut h = flate2::Crc::new();
                h.update(d);
                h.sum() as u64
            }
            Algo::Crc32c => {
                let mut c = !0u32;
                for &b in d {
                    c ^= b as u32;
                    for _ in 0..8 {
                        c = if c & 1 != 0 {
                            (c >> 1) ^ 0x82F6_3B78
                        } else {
                            c >> 1
                        };
                    }
                }
                !c as u64
            }
            Algo::Adler32 => {
                let (mut a, mut b) = (1u32, 0u32);
                for chunk in d.chunks(5552) {
                    for &x in chunk {
                        a += x as u32;
                        b += a;
                    }
                    a %= 65521;
                    b %= 65521;
                }
                ((b << 16) | a) as u64
            }
            Algo::Fletcher32 => {
                let (mut a, mut b) = (0u32, 0u32);
                for w in d.chunks(2) {
                    let v = u16::from_le_bytes([w[0], *w.get(1).unwrap_or(&0)]) as u32;
                    a = (a + v) % 65535;
                    b = (b + a) % 65535;
                }
                ((b << 16) | a) as u64
            }
            Algo::Fnv1a32 => d.iter().fold(0x811C_9DC5u32, |h, &b| {
                (h ^ b as u32).wrapping_mul(0x0100_0193)
            }) as u64,
            Algo::Sum32 => d.iter().fold(0u32, |s, &b| s.wrapping_add(b as u32)) as u64,
            Algo::Words32 => d.chunks(4).fold(0u32, |s, w| {
                let mut b = [0u8; 4];
                b[..w.len()].copy_from_slice(w);
                s.wrapping_add(u32::from_le_bytes(b))
            }) as u64,
            Algo::Crc16Ccitt => {
                let mut c = 0xFFFFu16;
                for &b in d {
                    c ^= (b as u16) << 8;
                    for _ in 0..8 {
                        c = if c & 0x8000 != 0 {
                            (c << 1) ^ 0x1021
                        } else {
                            c << 1
                        };
                    }
                }
                c as u64
            }
            Algo::Crc16Arc => {
                let mut c = 0u16;
                for &b in d {
                    c ^= b as u16;
                    for _ in 0..8 {
                        c = if c & 1 != 0 {
                            (c >> 1) ^ 0xA001
                        } else {
                            c >> 1
                        };
                    }
                }
                c as u64
            }
            Algo::Sum16 => d.iter().fold(0u16, |s, &b| s.wrapping_add(b as u16)) as u64,
            Algo::Sum8 => d.iter().fold(0u8, |s, &b| s.wrapping_add(b)) as u64,
            Algo::Xor8 => d.iter().fold(0u8, |s, &b| s ^ b) as u64,
            _ => 0,
        }
    }

    /// The bytes stored for `d`'s checksum.
    fn bytes(self, d: &[u8], be: bool) -> Vec<u8> {
        match self {
            Algo::Md5 => Md5::digest(d).to_vec(),
            Algo::Sha1 => Sha1::digest(d).to_vec(),
            Algo::Sha256 => Sha256::digest(d).to_vec(),
            _ => {
                let w = self.width();
                let v = self.value(d).to_le_bytes();
                let mut b = v[..w].to_vec();
                if be {
                    b.reverse();
                }
                b
            }
        }
    }
}

/// Which bytes the checksum covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Span {
    /// From just after the stored value to the end.
    After,
    /// From the start to just before the stored value.
    Before,
    /// The whole file, with the stored value's bytes counted as zero.
    Zeroed,
}

/// A checksum found in a file: what, where, and over which bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sum {
    pub algo: Algo,
    pub at: usize,
    pub be: bool,
    pub span: Span,
}

impl Sum {
    fn compute(&self, d: &[u8]) -> Option<Vec<u8>> {
        let w = self.algo.width();
        if self.at + w > d.len() {
            return None;
        }
        Some(match self.span {
            Span::After => self.algo.bytes(&d[self.at + w..], self.be),
            Span::Before => self.algo.bytes(&d[..self.at], self.be),
            Span::Zeroed => {
                let mut z = d.to_vec();
                z[self.at..self.at + w].fill(0);
                self.algo.bytes(&z, self.be)
            }
        })
    }

    /// True when `d` holds this checksum.
    pub fn holds(&self, d: &[u8]) -> bool {
        self.compute(d)
            .is_some_and(|v| d[self.at..self.at + v.len()] == v[..])
    }

    /// `d` with the checksum written again.
    pub fn fix(&self, d: &mut [u8]) -> Result<(), String> {
        let v = self
            .compute(d)
            .ok_or("the file is too short for its checksum")?;
        d[self.at..self.at + v.len()].copy_from_slice(&v);
        Ok(())
    }

    pub fn describe(&self) -> String {
        let span = match self.span {
            Span::After => "the bytes after it",
            Span::Before => "the bytes before it",
            Span::Zeroed => "the whole file (with itself as zero)",
        };
        let order = if self.algo.width() > 1 && !self.algo.is_hash() {
            if self.be {
                ", big-endian"
            } else {
                ", little-endian"
            }
        } else {
            ""
        };
        format!(
            "{} at 0x{:X}{order}, over {span}",
            self.algo.name(),
            self.at
        )
    }
}

/// Every checksum that holds in `d` (and in each of `others`, earlier copies of the same file,
/// which weeds out chance matches).
pub fn find(d: &[u8], others: &[Vec<u8>]) -> Vec<Sum> {
    let n = d.len();
    if !(8..=32 << 20).contains(&n) {
        return vec![];
    }
    let step = if n <= 1 << 20 { 1 } else { 4 };
    let near = |w: usize| -> Vec<usize> {
        let mut v: Vec<usize> = (0..=64.min(n - w)).step_by(step).collect();
        v.extend((0..=64.min(n - w)).step_by(step).map(|k| n - w - k));
        v.sort_unstable();
        v.dedup();
        v
    };
    let mut out = vec![];
    for algo in ALGOS {
        let w = algo.width();
        if w >= n {
            continue;
        }
        // Narrow sums match by chance too easily anywhere but the very first or last byte.
        let places = if algo.is_hash() || w == 1 {
            vec![0, n - w]
        } else {
            near(w)
        };
        for &at in &places {
            for span in [Span::After, Span::Before, Span::Zeroed] {
                if (span == Span::After && at + w == n) || (span == Span::Before && at == 0) {
                    continue;
                }
                let stored = &d[at..at + w];
                if stored.iter().all(|&b| b == 0) || stored.iter().all(|&b| b == 0xFF) {
                    continue;
                }
                for be in if w == 1 || algo.is_hash() {
                    vec![false]
                } else {
                    vec![false, true]
                } {
                    let s = Sum { algo, at, be, span };
                    if s.holds(d)
                        && others
                            .iter()
                            .filter(|o| o.len() == n && o[..] != d[..])
                            .all(|o| s.holds(o))
                    {
                        out.push(s);
                    }
                }
            }
        }
    }
    out
}

/// Fix-ups chosen for files, kept in the settings folder so they apply whenever the file opens.
#[cfg(not(target_arch = "wasm32"))]
fn store() -> Option<std::path::PathBuf> {
    crate::platform::config_dir().map(|d| d.join("checksums.txt"))
}

#[cfg(not(target_arch = "wasm32"))]
fn key(path: &std::path::Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string()
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn encode_line(s: &Sum) -> String {
    let algo = ALGOS.iter().position(|a| *a == s.algo).unwrap_or(0);
    let span = match s.span {
        Span::After => 0,
        Span::Before => 1,
        Span::Zeroed => 2,
    };
    format!("{algo}\t{}\t{}\t{span}", s.at, s.be as u8)
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn decode_line(t: &str) -> Option<Sum> {
    let v: Vec<usize> = t
        .split('\t')
        .map(|x| x.parse().ok())
        .collect::<Option<_>>()?;
    let [algo, at, be, span] = v[..] else {
        return None;
    };
    let span = [Span::After, Span::Before, Span::Zeroed]
        .get(span)
        .copied()?;
    Some(Sum {
        algo: *ALGOS.get(algo)?,
        at,
        be: be == 1,
        span,
    })
}

/// The fix-up chosen earlier for this file, if any.
#[cfg(not(target_arch = "wasm32"))]
pub fn remembered(path: &std::path::Path) -> Option<Sum> {
    let k = key(path);
    let text = std::fs::read_to_string(store()?).ok()?;
    text.lines().rev().find_map(|l| {
        l.strip_prefix(&k)
            .and_then(|r| r.strip_prefix('\t'))
            .and_then(decode_line)
    })
}

/// Remembers (or with `None`, forgets) the fix-up for this file.
#[cfg(not(target_arch = "wasm32"))]
pub fn remember(path: &std::path::Path, sum: Option<Sum>) -> Result<(), String> {
    let file = store().ok_or("no settings folder")?;
    let k = key(path);
    let old = std::fs::read_to_string(&file).unwrap_or_default();
    let mut lines: Vec<String> = old
        .lines()
        .filter(|l| !l.starts_with(&format!("{k}\t")))
        .map(String::from)
        .collect();
    if let Some(s) = sum {
        lines.push(format!("{k}\t{}", encode_line(&s)));
    }
    if let Some(d) = file.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::write(&file, lines.join("\n") + "\n").map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_and_fixes() {
        let base: Vec<u8> = (0..3000u32).map(|i| (i * 31 % 251) as u8).collect();
        for s in [
            Sum {
                algo: Algo::Crc32,
                at: base.len() - 4,
                be: false,
                span: Span::Before,
            },
            Sum {
                algo: Algo::Crc16Ccitt,
                at: 8,
                be: true,
                span: Span::Zeroed,
            },
            Sum {
                algo: Algo::Adler32,
                at: 0,
                be: false,
                span: Span::After,
            },
        ] {
            let mut d = base.clone();
            s.fix(&mut d).unwrap();
            let mut older = d.clone();
            older[500] ^= 0x10;
            s.fix(&mut older).unwrap();
            let found = find(&d, &[older]);
            assert_eq!(found, vec![s]);
            d[100] ^= 1;
            assert!(!s.holds(&d));
            s.fix(&mut d).unwrap();
            assert!(s.holds(&d));
        }
    }

    #[test]
    fn known_values() {
        assert_eq!(Algo::Crc32.value(b"123456789"), 0xCBF4_3926);
        assert_eq!(Algo::Crc32c.value(b"123456789"), 0xE306_9283);
        assert_eq!(Algo::Adler32.value(b"Wikipedia"), 0x11E6_0398);
        assert_eq!(Algo::Crc16Ccitt.value(b"123456789"), 0x29B1);
        assert_eq!(Algo::Crc16Arc.value(b"123456789"), 0xBB3D);
        assert_eq!(Algo::Fnv1a32.value(b"a"), 0xE40C_292C);
        assert_eq!(Algo::Fletcher32.value(b"abcde"), 0xF04F_C729);
    }

    #[test]
    fn line_round_trip() {
        let s = Sum {
            algo: Algo::Fnv1a32,
            at: 12,
            be: true,
            span: Span::Zeroed,
        };
        assert_eq!(decode_line(&encode_line(&s)), Some(s));
    }

    #[test]
    fn hashes() {
        let mut d = vec![7u8; 500];
        let s = Sum {
            algo: Algo::Sha1,
            at: 0,
            be: false,
            span: Span::After,
        };
        s.fix(&mut d).unwrap();
        assert!(find(&d, &[]).contains(&s));
    }
}
