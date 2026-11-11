//! Finding and repacking zlib/deflate streams inside a save, for the patch scripts'
//! `decompress` and `compress` commands. Ported from apollo-lib's `offzip.c` and `packzip.c`
//! (Luigi Auriemma's offzip/packzip, GPL-2.0-or-later, as adapted by Damian Parrino).

use flate2::{Decompress, FlushDecompress, Status};

pub const WBITS_ZLIB: i32 = 15;
pub const WBITS_DEFLATE: i32 = -15;
const MIN_ZIP: usize = 32;
const IN_SIZE: usize = 0x800;
const OUT_SIZE: usize = 0x10000;
const MAX_RESULTS: usize = 0x40;

/// One stream found in the data.
#[derive(Clone, Debug)]
pub struct Found {
    pub offset: usize,
    pub ziplen: usize,
    pub wbits: i32,
    pub data: Vec<u8>,
}

fn decoder(wbits: i32) -> Decompress {
    Decompress::new(wbits > 0)
}

/// Whether a stream starts at `at`: inflating its first 32 bytes makes progress without
/// ending or failing.
fn starts(z: &mut Decompress, out: &mut [u8], d: &[u8], at: usize, wbits: i32) -> bool {
    z.reset(wbits > 0);
    matches!(z.decompress(&d[at..at + MIN_ZIP], out, FlushDecompress::Sync), Ok(Status::Ok))
}

/// Inflates from `at`, 0x800 bytes of input at a time. A stream that breaks off after some
/// progress still counts, as in offzip. Returns (bytes read, output).
fn unzip(d: &[u8], at: usize, wbits: i32) -> Option<(usize, Vec<u8>)> {
    let mut z = decoder(wbits);
    let mut out = vec![];
    let mut buf = vec![0u8; OUT_SIZE];
    let mut ok = false;
    let mut pos = at;
    'outer: while pos < d.len() {
        let chunk = &d[pos..(pos + IN_SIZE).min(d.len())];
        let mut used = 0;
        loop {
            let (i0, o0) = (z.total_in(), z.total_out());
            let r = z.decompress(&chunk[used..], &mut buf, FlushDecompress::Sync);
            used += (z.total_in() - i0) as usize;
            out.extend_from_slice(&buf[..(z.total_out() - o0) as usize]);
            match r {
                Ok(Status::Ok) => ok = true,
                Ok(Status::StreamEnd) => {
                    ok = true;
                    break 'outer;
                }
                _ => break 'outer,
            }
            if used >= chunk.len() {
                break;
            }
        }
        pos += chunk.len();
    }
    ok.then(|| (z.total_in() as usize, out))
}

/// The streams from `offset` on: at most `count` of them (all, up to 64, when 0).
pub fn scan(d: &[u8], offset: usize, wbits: i32, count: usize) -> Vec<Found> {
    let mut found = vec![];
    let limit = if count == 0 { MAX_RESULTS } else { count };
    let mut at = offset;
    let mut z = decoder(wbits);
    let mut buf = vec![0u8; OUT_SIZE];
    while found.len() < limit {
        // Search for the next place a stream starts.
        while at + MIN_ZIP <= d.len() && !starts(&mut z, &mut buf, d, at, wbits) {
            at += 1;
        }
        if at + MIN_ZIP > d.len() {
            break;
        }
        match unzip(d, at, wbits) {
            Some((inlen, data)) => {
                found.push(Found { offset: at, ziplen: inlen, wbits, data });
                at += inlen.max(1);
            }
            None => at += 1,
        }
    }
    found
}

/// Packs `data` again (best compression) and writes it where the stream was found, zeroing
/// the old stream first; the file grows if the new stream runs past its end.
pub fn pack(f: &mut Found, data: &[u8], out: &mut Vec<u8>) -> bool {
    use std::io::Write;
    let level = flate2::Compression::best();
    let z = if f.wbits > 0 {
        let mut e = flate2::write::ZlibEncoder::new(vec![], level);
        e.write_all(data).ok().and_then(|_| e.finish().ok())
    } else {
        let mut e = flate2::write::DeflateEncoder::new(vec![], level);
        e.write_all(data).ok().and_then(|_| e.finish().ok())
    };
    let Some(z) = z else { return false };
    if out.len() > f.offset {
        let end = (f.offset + f.ziplen).min(out.len());
        out[f.offset..end].fill(0);
    }
    if out.len() < f.offset + z.len() {
        out.resize(f.offset + z.len(), 0);
    }
    out[f.offset..f.offset + z.len()].copy_from_slice(&z);
    f.ziplen = z.len();
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn finds_and_repacks() {
        let payload: Vec<u8> = (0..4000u32).map(|i| (i % 251) as u8).collect();
        let mut e = flate2::write::ZlibEncoder::new(vec![], flate2::Compression::default());
        e.write_all(&payload).unwrap();
        let z = e.finish().unwrap();
        let mut file = vec![0xAAu8; 0x18];
        file.extend(&z);
        file.extend([0x55u8; 40]);
        let f = scan(&file, 0, WBITS_ZLIB, 0);
        assert_eq!(f.len(), 1);
        assert_eq!((f[0].offset, f[0].ziplen), (0x18, z.len()));
        assert_eq!(f[0].data, payload);
        let mut g = f[0].clone();
        let mut edited = payload.clone();
        edited[10] = 0xEE;
        let mut out = file.clone();
        assert!(pack(&mut g, &edited, &mut out));
        assert_eq!(scan(&out, 0, WBITS_ZLIB, 1)[0].data, edited);
    }
}
