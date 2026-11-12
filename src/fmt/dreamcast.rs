//! Dreamcast VMU saves as PC tools keep them: a `.VMS` file (the file's own blocks, with
//! the VMS header and its CRC) and a Nexus `.DCI` file (the card's directory entry, then the
//! blocks with every 4 bytes reversed). Layouts after Marcus Comstedt's Dreamcast notes and
//! Apollo DC's dcsave.c (GPL-3.0, Damian Parrino).

const CRC_AT: usize = 0x46;
const ICON_SIZE: usize = 512;
const DCI_HEADER: usize = 32;
const BLOCK: usize = 512;

fn le16(d: &[u8], at: usize) -> usize {
    u16::from_le_bytes([d[at], d[at + 1]]) as usize
}

/// The span the CRC covers (header, icons, eyecatch and payload), if the header at `off`
/// reads.
fn vms_span(d: &[u8], off: usize) -> Option<usize> {
    let h = d.get(off..off + 0x80)?;
    let icons = le16(h, 0x40);
    let eyecatch = le16(h, 0x44);
    if icons == 0 || icons > 16 || eyecatch > 3 {
        return None;
    }
    let eye = [0, 72 * 56 * 2, 512 + 72 * 56, 32 + 72 * 56 / 2][eyecatch];
    let payload = u32::from_le_bytes(h[0x48..0x4C].try_into().ok()?) as usize;
    let n = 0x80 + icons * ICON_SIZE + eye + payload;
    (off + n <= d.len()).then_some(n)
}

fn crc(d: &[u8], off: usize, n: usize) -> u16 {
    let mut c: u16 = 0;
    for i in 0..n {
        let b = if i == CRC_AT || i == CRC_AT + 1 { 0 } else { d[off + i] };
        c ^= (b as u16) << 8;
        for _ in 0..8 {
            c = if c & 0x8000 != 0 { (c << 1) ^ 0x1021 } else { c << 1 };
        }
    }
    c
}

/// Where a VMS file's header is (0 for data files, the second block for games), when its
/// CRC checks out.
pub fn vms_header(d: &[u8]) -> Option<usize> {
    [0, BLOCK].into_iter().find(|&off| {
        vms_span(d, off).is_some_and(|n| d[off + CRC_AT..off + CRC_AT + 2] != [0, 0] && crc(d, off, n) == le16(d, off + CRC_AT) as u16)
    })
}

/// The data with its CRC shown as zeros (it is written again on save).
pub fn vms_open(d: &[u8], off: usize) -> Vec<u8> {
    let mut v = d.to_vec();
    v[off + CRC_AT..off + CRC_AT + 2].fill(0);
    v
}

pub fn vms_close(d: &[u8], off: usize) -> Result<Vec<u8>, String> {
    let n = vms_span(d, off).ok_or("the VMS header no longer reads (its icon count or size changed)")?;
    let mut v = d.to_vec();
    let c = crc(&v, off, n);
    v[off + CRC_AT..off + CRC_AT + 2].copy_from_slice(&c.to_le_bytes());
    Ok(v)
}

fn swap32(d: &mut [u8]) {
    for w in d.chunks_exact_mut(4) {
        w.reverse();
    }
}

/// A Nexus `.DCI`: the directory entry and the file's blocks, unswapped.
pub fn dci_open(d: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if d.len() < DCI_HEADER + BLOCK || !matches!(d[0], 0x33 | 0xCC) {
        return None;
    }
    let n = le16(d, 0x18) * BLOCK;
    if n == 0 || d.len() != DCI_HEADER + n {
        return None;
    }
    let mut data = d[DCI_HEADER..].to_vec();
    swap32(&mut data);
    Some((d[..DCI_HEADER].to_vec(), data))
}

pub fn dci_close(header: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() != le16(header, 0x18) * BLOCK {
        return Err("a Dreamcast save keeps its size in whole 512-byte blocks".into());
    }
    let mut out = header.to_vec();
    let mut body = data.to_vec();
    swap32(&mut body);
    out.extend(body);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vms_crc_and_dci() {
        let mut d = vec![0u8; 3 * BLOCK];
        d[0..4].copy_from_slice(b"TEST");
        d[0x40] = 1;
        d[0x48..0x4C].copy_from_slice(&100u32.to_le_bytes());
        let c = crc(&d, 0, vms_span(&d, 0).unwrap());
        d[CRC_AT..CRC_AT + 2].copy_from_slice(&c.to_le_bytes());
        assert_eq!(vms_header(&d), Some(0));
        let mut v = vms_open(&d, 0);
        assert_eq!(vms_close(&v, 0).unwrap(), d);
        v[0x300] = 9;
        assert_eq!(vms_header(&vms_close(&v, 0).unwrap()), Some(0));

        let mut h = vec![0u8; DCI_HEADER];
        h[0] = 0x33;
        h[0x18] = 3;
        let mut sw = d.clone();
        swap32(&mut sw);
        let dci = [h.clone(), sw].concat();
        let (hh, data) = dci_open(&dci).unwrap();
        assert_eq!(data, d);
        assert_eq!(dci_close(&hh, &data).unwrap(), dci);
    }
}
