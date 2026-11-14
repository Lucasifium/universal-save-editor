//! Single PlayStation 1 and 2 saves as cheat devices and PC tools export them: `.mcs`,
//! `.psx`, PS3 `.psv` (signed again on save), uLaunchELF `.psu`, Action Replay Max `.max`
//! (LZARI), CodeBreaker `.cbs` (RC4 and zlib) and Xploder/SharkPort `.xps`.
//!
//! Layouts and the `.psv` signature are ported from psv-save-converter (GPL-3.0, Damian
//! Parrino), after ps3-psvresigner (dots-tb), CheatDevicePS2 (root670) and PSV-Exporter
//! (PMStanley).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Mcs,
    Psx,
    Psv1,
    Psv2,
    Psu,
    Max,
    Cbs,
    Xps,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Mcs => "PlayStation save (.mcs)",
            Kind::Psx => "PlayStation save (.psx, Action Replay / GameShark)",
            Kind::Psv1 => "PlayStation save for PS3 (.psv)",
            Kind::Psv2 => "PlayStation 2 save for PS3 (.psv)",
            Kind::Psu => "PlayStation 2 save (.psu, uLaunchELF)",
            Kind::Max => "PlayStation 2 save (.max, Action Replay Max)",
            Kind::Cbs => "PlayStation 2 save (.cbs, CodeBreaker)",
            Kind::Xps => "PlayStation 2 save (.xps, Xploder / SharkPort)",
        }
    }
}

/// One file of the save: `DIR/name` for PS2 saves, the product code for PS1 ones.
#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub data: Vec<u8>,
}

fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn put32(d: &mut [u8], at: usize, v: u32) {
    d[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

fn cstr(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).into_owned()
}

fn join(dir: &str, name: &str) -> String {
    format!("{dir}/{name}")
}

// --- Detection and reading ---

const PSV_MAGIC: &[u8] = b"\0VSP";
const MAX_MAGIC: &[u8] = b"Ps2PowerSave";
const CBS_MAGIC: &[u8] = b"CFU\0";
const XPS_MAGIC: &[u8] = b"\x0d\0\0\0SharkPortSave";
const MAX_HEADER: usize = 0x5C;
const CBS_HEADER: usize = 0x128;
const CBS_ENTRY: usize = 64;
const XPS_ENTRY_MIN: usize = 70;
const PSU_ENTRY: usize = 512;

fn mcs_ok(d: &[u8]) -> bool {
    d.len() > 0x80 && d[0] == 0x51 && (d.len() - 0x80) % 0x2000 == 0 && d[..0x7F].iter().fold(0, |a, b| a ^ b) == d[0x7F] && d[0x80..0x82] == *b"SC"
}

/// Recognises a single save and lists its files.
pub fn open(d: &[u8], ext: &str) -> Option<(Kind, Vec<Entry>)> {
    if d.starts_with(PSV_MAGIC) && d.len() > 0x84 {
        return match d[0x3C] {
            1 => Some((Kind::Psv1, vec![Entry { name: cstr(&d[0x64..0x78]), data: d[0x84..].to_vec() }])),
            2 => Some((Kind::Psv2, psv2_files(d)?)),
            _ => None,
        };
    }
    if d.starts_with(MAX_MAGIC) {
        return Some((Kind::Max, max_files(d)?));
    }
    if d.starts_with(CBS_MAGIC) {
        return Some((Kind::Cbs, cbs_files(d)?));
    }
    // "SharkPortSave" with its length in front, then a version (0 in X-Port files, 2 in SharkPort ones).
    if d.get(..17) == Some(XPS_MAGIC) {
        return Some((Kind::Xps, xps_files(d)?.0));
    }
    if mcs_ok(d) {
        return Some((Kind::Mcs, vec![Entry { name: cstr(&d[0x0A..0x1E]), data: d[0x80..].to_vec() }]));
    }
    if ext == "psx" && d.len() > 0x38 && d[0x36..0x38] == *b"SC" {
        return Some((Kind::Psx, vec![Entry { name: cstr(&d[..0x14]), data: d[0x36..].to_vec() }]));
    }
    if let Some(files) = psu_files(d) {
        return Some((Kind::Psu, files));
    }
    None
}

fn psv2_files(d: &[u8]) -> Option<Vec<Entry>> {
    let n = le32(d, 0x64)? as usize;
    let dir = cstr(d.get(0x68 + 24..0x68 + 56)?);
    if n == 0 || n > 4096 {
        return None;
    }
    (0..n)
        .map(|i| {
            let at = 0xA0 + i * 0x3C;
            let size = le32(d, at + 16)? as usize;
            let pos = le32(d, at + 56)? as usize;
            Some(Entry { name: join(&dir, &cstr(d.get(at + 24..at + 56)?)), data: d.get(pos..pos.checked_add(size)?)?.to_vec() })
        })
        .collect()
}

fn psu_files(d: &[u8]) -> Option<Vec<Entry>> {
    if d.len() < PSU_ENTRY * 3 || d.len() % PSU_ENTRY != 0 {
        return None;
    }
    let mode = u16::from_le_bytes([d[0], d[1]]);
    if mode & 0x20 == 0 || cstr(&d[PSU_ENTRY + 64..PSU_ENTRY + 96]) != "." || cstr(&d[2 * PSU_ENTRY + 64..2 * PSU_ENTRY + 96]) != ".." {
        return None;
    }
    let dir = cstr(&d[64..96]);
    let count = le32(d, 4)?.checked_sub(2)? as usize;
    let mut at = 3 * PSU_ENTRY;
    let mut out = vec![];
    for _ in 0..count {
        let len = le32(d, at + 4)? as usize;
        let name = cstr(d.get(at + 64..at + 96)?);
        at += PSU_ENTRY;
        out.push(Entry { name: join(&dir, &name), data: d.get(at..at.checked_add(len)?)?.to_vec() });
        at += len.div_ceil(1024) * 1024;
    }
    (!dir.is_empty()).then_some(out)
}

fn max_next(off: usize) -> usize {
    off + (16 - ((off + 8) % 16)) % 16
}

fn max_files(d: &[u8]) -> Option<Vec<Entry>> {
    if d.len() < MAX_HEADER {
        return None;
    }
    let dir = cstr(&d[16..48]);
    let n = le32(d, 0x54)? as usize;
    let raw = super::lzari::decompress(&d[MAX_HEADER - 4..])?;
    let mut at = 0;
    let mut out = vec![];
    for _ in 0..n {
        let len = le32(&raw, at)? as usize;
        let name = cstr(raw.get(at + 4..at + 36)?);
        at += 36;
        out.push(Entry { name: join(&dir, &name), data: raw.get(at..at.checked_add(len)?)?.to_vec() });
        at = max_next(at + len);
    }
    (n > 0).then_some(out)
}

/// The fixed RC4 state CodeBreaker saves are encrypted with (as in mymc's ps2save.py).
const CBS_KEY: [u8; 256] = [
    0x5f, 0x1f, 0x85, 0x6f, 0x31, 0xaa, 0x3b, 0x18, 0x21, 0xb9, 0xce, 0x1c, 0x07, 0x4c, 0x9c, 0xb4, 0x81, 0xb8, 0xef, 0x98, 0x59, 0xae, 0xf9, 0x26, 0xe3, 0x80, 0xa3,
    0x29, 0x2d, 0x73, 0x51, 0x62, 0x7c, 0x64, 0x46, 0xf4, 0x34, 0x1a, 0xf6, 0xe1, 0xba, 0x3a, 0x0d, 0x82, 0x79, 0x0a, 0x5c, 0x16, 0x71, 0x49, 0x8e, 0xac, 0x8c, 0x9f,
    0x35, 0x19, 0x45, 0x94, 0x3f, 0x56, 0x0c, 0x91, 0x00, 0x0b, 0xd7, 0xb0, 0xdd, 0x39, 0x66, 0xa1, 0x76, 0x52, 0x13, 0x57, 0xf3, 0xbb, 0x4e, 0xe5, 0xdc, 0xf0, 0x65,
    0x84, 0xb2, 0xd6, 0xdf, 0x15, 0x3c, 0x63, 0x1d, 0x89, 0x14, 0xbd, 0xd2, 0x36, 0xfe, 0xb1, 0xca, 0x8b, 0xa4, 0xc6, 0x9e, 0x67, 0x47, 0x37, 0x42, 0x6d, 0x6a, 0x03,
    0x92, 0x70, 0x05, 0x7d, 0x96, 0x2f, 0x40, 0x90, 0xc4, 0xf1, 0x3e, 0x3d, 0x01, 0xf7, 0x68, 0x1e, 0xc3, 0xfc, 0x72, 0xb5, 0x54, 0xcf, 0xe7, 0x41, 0xe4, 0x4d, 0x83,
    0x55, 0x12, 0x22, 0x09, 0x78, 0xfa, 0xde, 0xa7, 0x06, 0x08, 0x23, 0xbf, 0x0f, 0xcc, 0xc1, 0x97, 0x61, 0xc5, 0x4a, 0xe6, 0xa0, 0x11, 0xc2, 0xea, 0x74, 0x02, 0x87,
    0xd5, 0xd1, 0x9d, 0xb7, 0x7e, 0x38, 0x60, 0x53, 0x95, 0x8d, 0x25, 0x77, 0x10, 0x5e, 0x9b, 0x7f, 0xd8, 0x6e, 0xda, 0xa2, 0x2e, 0x20, 0x4f, 0xcd, 0x8f, 0xcb, 0xbe,
    0x5a, 0xe0, 0xed, 0x2c, 0x9a, 0xd4, 0xe2, 0xaf, 0xd0, 0xa9, 0xe8, 0xad, 0x7a, 0xbc, 0xa8, 0xf2, 0xee, 0xeb, 0xf5, 0xa6, 0x99, 0x28, 0x24, 0x6c, 0x2b, 0x75, 0x5d,
    0xf8, 0xd3, 0x86, 0x17, 0xfb, 0xc0, 0x7b, 0xb3, 0x58, 0xdb, 0xc7, 0x4b, 0xff, 0x04, 0x50, 0xe9, 0x88, 0x69, 0xc9, 0x2a, 0xab, 0xfd, 0x5b, 0x1b, 0x8a, 0xd9, 0xec,
    0x27, 0x44, 0x0e, 0x33, 0xc8, 0x6b, 0x93, 0x32, 0x48, 0xb6, 0x30, 0x43, 0xa5,
];

fn rc4(buf: &mut [u8]) {
    let mut s = CBS_KEY;
    let (mut j, mut k) = (0u8, 0u8);
    for b in buf {
        j = j.wrapping_add(1);
        k = k.wrapping_add(s[j as usize]);
        s.swap(j as usize, k as usize);
        *b ^= s[(s[j as usize].wrapping_add(s[k as usize])) as usize];
    }
}

fn cbs_files(d: &[u8]) -> Option<Vec<Entry>> {
    if d.len() < CBS_HEADER {
        return None;
    }
    let dir = cstr(&d[0x14..0x34]);
    let start = le32(d, 8)? as usize;
    let mut z = d.get(start..)?.to_vec();
    rc4(&mut z);
    let mut raw = vec![];
    std::io::Read::read_to_end(&mut flate2::read::ZlibDecoder::new(&z[..]), &mut raw).ok()?;
    let mut at = 0;
    let mut out = vec![];
    while at + CBS_ENTRY <= raw.len() {
        let len = le32(&raw, at + 16)? as usize;
        let name = cstr(&raw[at + 32..at + 64]);
        at += CBS_ENTRY;
        out.push(Entry { name: join(&dir, &name), data: raw.get(at..at.checked_add(len)?)?.to_vec() });
        at += len;
    }
    (!out.is_empty()).then_some(out)
}

/// Where the body starts (after the header's three strings and the body length).
fn xps_body(d: &[u8]) -> Option<usize> {
    let mut at: usize = 0x15;
    for _ in 0..3 {
        at = at.checked_add(4 + le32(d, at)? as usize)?;
    }
    Some(at + 4)
}

/// An entry header at `at`: its own size comes first (250 bytes in X-Port files, 98 in
/// SharkPort ones); the name and length are at the same places in both.
fn xps_entry(d: &[u8], at: usize) -> Option<&[u8]> {
    let n = u16::from_le_bytes(d.get(at..at + 2)?.try_into().ok()?) as usize;
    if n < XPS_ENTRY_MIN {
        return None;
    }
    d.get(at..at + n)
}

fn xps_files(d: &[u8]) -> Option<(Vec<Entry>, usize)> {
    let body = xps_body(d)?;
    let dir_entry = xps_entry(d, body)?;
    let dir = cstr(&dir_entry[2..66]);
    let count = le32(dir_entry, 66)?.checked_sub(2)? as usize;
    let mut at = body + dir_entry.len();
    let mut out = vec![];
    for _ in 0..count {
        let e = xps_entry(d, at)?;
        let len = le32(e, 66)? as usize;
        at += e.len();
        out.push(Entry { name: join(&dir, &cstr(&e[2..66])), data: d.get(at..at.checked_add(len)?)?.to_vec() });
        at += len;
    }
    Some((out, at))
}

fn xps_sum(body: &[u8]) -> u32 {
    let mut sum: u32 = 0;
    for &b in body {
        sum = sum.wrapping_add((b as u32) << (sum % 24));
    }
    sum
}

// --- Writing ---

/// Writes `data` as the file `name` of the save `original`, keeping everything else.
pub fn save(kind: Kind, original: &[u8], name: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    let (_, files) = open(original, if kind == Kind::Psx { "psx" } else { "" }).ok_or("the original save no longer reads")?;
    let idx = files.iter().position(|f| f.name == name).ok_or_else(|| format!("{name} isn't in this save"))?;
    let same = files[idx].data.len() == data.len();
    // Unchanged data keeps the file exactly as it was (repacking needn't give the same bytes).
    if files[idx].data == data && !matches!(kind, Kind::Psv1 | Kind::Psv2) {
        return Ok(original.to_vec());
    }
    let mut files = files;
    files[idx].data = data.to_vec();
    // .hack keeps each slot's checksum and preview in the folder's system file.
    let mut other_changed = false;
    if super::dothack::is_slot_file(name) {
        let folder = name.rsplit_once('/').map_or("", |x| x.0).to_string();
        if let Some(s) = files.iter().position(|f| super::dothack::is_system_file(&f.name) && f.name.starts_with(&folder)) {
            if let Some(n) = super::dothack::system_after(name, &files[s].data, data) {
                other_changed = n != files[s].data;
                files[s].data = n;
            }
        }
    }
    Ok(match kind {
        Kind::Mcs => {
            if data.is_empty() || data.len() % 0x2000 != 0 {
                return Err("a PlayStation save is made of whole 8 KB blocks".into());
            }
            let mut h = original[..0x80].to_vec();
            if !same {
                // The size sits at 0x04, and some writers repeat it at 0x38.
                let old = le32(&h, 4);
                put32(&mut h, 4, data.len() as u32);
                if le32(&h, 0x38) == old {
                    put32(&mut h, 0x38, data.len() as u32);
                }
                h[0x7F] = h[..0x7F].iter().fold(0, |a, b| a ^ b);
            }
            [h, data.to_vec()].concat()
        }
        Kind::Psx => [&original[..0x36], data].concat(),
        Kind::Psv1 => {
            let mut out = [&original[..0x84], data].concat();
            if !same {
                let old = le32(&out, 0x40);
                put32(&mut out, 0x40, data.len() as u32);
                if le32(&out, 0x5C) == old {
                    put32(&mut out, 0x5C, data.len() as u32);
                }
            }
            psv_sign(&mut out);
            out
        }
        Kind::Psv2 => {
            let mut out = if same && !other_changed {
                let at = 0xA0 + idx * 0x3C;
                let pos = le32(original, at + 56).unwrap_or_default() as usize;
                let mut o = original.to_vec();
                o[pos..pos + data.len()].copy_from_slice(data);
                o
            } else {
                psv2_rebuild(original, &files)
            };
            psv_sign(&mut out);
            out
        }
        Kind::Psu => {
            let mut out = original[..3 * PSU_ENTRY].to_vec();
            let mut at = 3 * PSU_ENTRY;
            for f in &files {
                let mut e = original[at..at + PSU_ENTRY].to_vec();
                let old = le32(&e, 4).unwrap_or_default() as usize;
                put32(&mut e, 4, f.data.len() as u32);
                out.extend(e);
                out.extend(&f.data);
                // Writers pad with 0xFF or with zeros; keep whichever this one used.
                let pad = original.get(at + PSU_ENTRY + old).copied().filter(|_| old % 1024 != 0).unwrap_or(0xFF);
                out.resize(out.len() + (1024 - f.data.len() % 1024) % 1024, pad);
                at += PSU_ENTRY + old.div_ceil(1024) * 1024;
            }
            out
        }
        Kind::Max => {
            let n = le32(original, 0x54).unwrap_or_default() as usize;
            let raw = super::lzari::decompress(&original[MAX_HEADER - 4..]).ok_or("the save's packed data no longer reads")?;
            let mut packed = vec![];
            let mut at = 0;
            for f in files.iter().take(n) {
                let len = le32(&raw, at).unwrap_or_default() as usize;
                packed.extend((f.data.len() as u32).to_le_bytes());
                packed.extend(&raw[at + 4..at + 36]);
                packed.extend(&f.data);
                packed.resize(max_next(packed.len()), 0);
                at = max_next(at + 36 + len);
            }
            let stream = super::lzari::compress(&packed);
            let old_stream = original.len() - (MAX_HEADER - 4);
            let old_size = le32(original, 0x50).unwrap_or_default() as usize;
            let mut out = original[..MAX_HEADER - 4].to_vec();
            put32(&mut out, 0x50, (old_size + stream.len()).saturating_sub(old_stream) as u32);
            out.extend(stream);
            if le32(original, 12) != Some(0) {
                put32(&mut out, 12, 0);
                let c = crc32(&out);
                put32(&mut out, 12, c);
            }
            out
        }
        Kind::Cbs => {
            let start = le32(original, 8).unwrap_or_default() as usize;
            let mut z = original[start..].to_vec();
            rc4(&mut z);
            let mut old = vec![];
            std::io::Read::read_to_end(&mut flate2::read::ZlibDecoder::new(&z[..]), &mut old).map_err(|e| e.to_string())?;
            let mut raw = vec![];
            let mut at = 0;
            for f in &files {
                let mut e = old[at..at + CBS_ENTRY].to_vec();
                let len = le32(&e, 16).unwrap_or_default() as usize;
                put32(&mut e, 16, f.data.len() as u32);
                raw.extend(e);
                raw.extend(&f.data);
                at += CBS_ENTRY + len;
            }
            let mut enc = flate2::write::ZlibEncoder::new(vec![], flate2::Compression::best());
            std::io::Write::write_all(&mut enc, &raw).map_err(|e| e.to_string())?;
            let mut z = enc.finish().map_err(|e| e.to_string())?;
            rc4(&mut z);
            let mut out = original[..start].to_vec();
            put32(&mut out, 0x0C, raw.len() as u32);
            put32(&mut out, 0x10, z.len() as u32);
            out.extend(z);
            out
        }
        Kind::Xps => {
            let body = xps_body(original).ok_or("the save's header no longer reads")?;
            let (_, end) = xps_files(original).ok_or("the save's files no longer read")?;
            let first = xps_entry(original, body).ok_or("the save's files no longer read")?.len();
            let mut out = original[..body + first].to_vec();
            let mut at = body + first;
            for f in &files {
                let mut e = xps_entry(original, at).ok_or("the save's files no longer read")?.to_vec();
                let (n, len) = (e.len(), le32(&e, 66).unwrap_or_default() as usize);
                put32(&mut e, 66, f.data.len() as u32);
                out.extend(e);
                out.extend(&f.data);
                at += n + len;
            }
            let body_len = (out.len() - body) as u32;
            put32(&mut out, body - 4, body_len);
            // The optional trailer: a sum of the body, or zero when the writer left it alone.
            if original.len() == end + 4 {
                let stored = le32(original, end).unwrap_or_default();
                let sum = if stored == 0 { 0 } else { xps_sum(&out[body..]) };
                out.extend(sum.to_le_bytes());
            }
            out
        }
    })
}

fn psv2_rebuild(original: &[u8], files: &[Entry]) -> Vec<u8> {
    let n = files.len();
    let mut head = original[..0xA0 + n * 0x3C].to_vec();
    let mut pos = head.len();
    let olds: Vec<(usize, usize)> = (0..n)
        .map(|i| (le32(original, 0xA0 + i * 0x3C + 56).unwrap_or_default() as usize, le32(original, 0xA0 + i * 0x3C + 16).unwrap_or_default() as usize))
        .collect();
    let mut news = vec![];
    for (i, f) in files.iter().enumerate() {
        let at = 0xA0 + i * 0x3C;
        put32(&mut head, at + 16, f.data.len() as u32);
        put32(&mut head, at + 56, pos as u32);
        news.push((pos, f.data.len()));
        pos += f.data.len();
    }
    put32(&mut head, 0x40, files.iter().map(|f| f.data.len() as u32).sum());
    // icon.sys and the three icons: (position, size) pairs after the display size.
    for k in 0..4 {
        let at = 0x44 + k * 8;
        let old = le32(original, at).unwrap_or_default() as usize;
        if let Some(i) = olds.iter().position(|o| o.0 == old && o.1 > 0) {
            put32(&mut head, at, news[i].0 as u32);
            put32(&mut head, at + 4, news[i].1 as u32);
        }
    }
    for f in files {
        head.extend(&f.data);
    }
    head
}

fn crc32(d: &[u8]) -> u32 {
    let mut c = !0u32;
    for &b in d {
        c ^= b as u32;
        for _ in 0..8 {
            c = (c >> 1) ^ (0xEDB8_8320 & (c & 1).wrapping_neg());
        }
    }
    !c
}

// --- The PS3 .psv signature ---

const PSV_KEYS: [[u8; 16]; 2] = [
    [0xEA, 0x02, 0xCE, 0xEF, 0x5B, 0xB4, 0xD2, 0x99, 0x8F, 0x61, 0x19, 0x10, 0xD7, 0x7F, 0x51, 0xC6],
    [0xAB, 0x5A, 0xBC, 0x9F, 0xC1, 0xF4, 0x9D, 0xE6, 0xA0, 0x51, 0xDB, 0xAE, 0xFA, 0x51, 0x88, 0x59],
];
const PSV_IV: [u8; 16] = [0xB3, 0x0F, 0xFE, 0xED, 0xB7, 0xDC, 0x5E, 0xB7, 0x13, 0x3D, 0xA6, 0x0D, 0x1B, 0x6B, 0x2C, 0xDC];

/// The signature a `.psv` should carry: HMAC-SHA1 over the file (signature zeroed), keyed
/// from the salt at 0x08.
pub fn psv_signature(d: &[u8]) -> Option<[u8; 20]> {
    use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
    use hmac::Mac;
    let seed = d.get(8..0x1C)?;
    let mut key = [0u8; 0x40];
    match d.get(0x3C)? {
        1 => {
            let c = aes::Aes128::new((&PSV_KEYS[1]).into());
            let mut a = aes::Block::clone_from_slice(&seed[..16]);
            c.decrypt_block(&mut a);
            let mut b = aes::Block::clone_from_slice(&seed[..16]);
            c.encrypt_block(&mut b);
            for i in 0..16 {
                key[i] = a[i] ^ PSV_IV[i];
                key[16 + i] = b[i] ^ if i < 4 { seed[16 + i] } else { 0xFF };
            }
        }
        2 => {
            // CBC decryption of the 64-byte salt block; only the first 20 bytes are kept.
            let c = aes::Aes128::new((&PSV_KEYS[0]).into());
            let mut buf = [0u8; 0x40];
            buf[..0x14].copy_from_slice(seed);
            let mut prev = PSV_IV;
            for blk in buf.chunks_mut(16) {
                let ct: [u8; 16] = blk.try_into().unwrap();
                let mut b = aes::Block::clone_from_slice(blk);
                c.decrypt_block(&mut b);
                for i in 0..16 {
                    blk[i] = b[i] ^ prev[i];
                }
                prev = ct;
            }
            key[..0x14].copy_from_slice(&buf[..0x14]);
        }
        _ => return None,
    }
    let mut m = <hmac::Hmac<sha1::Sha1> as Mac>::new_from_slice(&key[..0x14]).ok()?;
    m.update(&d[..0x1C]);
    m.update(&[0; 0x14]);
    m.update(d.get(0x30..)?);
    Some(m.finalize().into_bytes().into())
}

fn psv_sign(d: &mut [u8]) {
    if let Some(s) = psv_signature(d) {
        d[0x1C..0x30].copy_from_slice(&s);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn psu(dir: &str, files: &[(&str, &[u8])]) -> Vec<u8> {
        let entry = |mode: u16, len: usize, name: &str| {
            let mut e = vec![0u8; PSU_ENTRY];
            e[..2].copy_from_slice(&mode.to_le_bytes());
            put32(&mut e, 4, len as u32);
            e[64..64 + name.len()].copy_from_slice(name.as_bytes());
            e
        };
        let mut out = entry(0x8427, files.len() + 2, dir);
        out.extend(entry(0x8427, 0, "."));
        out.extend(entry(0x8427, 0, ".."));
        for (n, d) in files {
            out.extend(entry(0x8497, d.len(), n));
            out.extend(*d);
            out.resize(out.len() + (1024 - d.len() % 1024) % 1024, 0xFF);
        }
        out
    }

    #[test]
    fn psu_round_trip() {
        let d = psu("BASLUS-20000GAME", &[("icon.sys", &[1; 964]), ("SAVE", &[7; 3000])]);
        let (k, f) = open(&d, "psu").unwrap();
        assert_eq!(k, Kind::Psu);
        assert_eq!(f[1].name, "BASLUS-20000GAME/SAVE");
        assert_eq!(save(k, &d, &f[1].name, &[7; 3000]).unwrap(), d);
        let grown = save(k, &d, &f[1].name, &[9; 5000]).unwrap();
        let (_, g) = open(&grown, "psu").unwrap();
        assert_eq!(g[1].data, vec![9; 5000]);
        assert_eq!(g[0].data, f[0].data);
    }

    #[test]
    fn max_and_cbs_round_trip() {
        // A .max made by hand: header, then the LZARI stream of the entries.
        let mut raw = vec![];
        for (n, d) in [("icon.sys", vec![1u8; 100]), ("DATA", (0..999u32).map(|i| (i * 7) as u8).collect())] {
            raw.extend((d.len() as u32).to_le_bytes());
            let mut name = [0u8; 32];
            name[..n.len()].copy_from_slice(n.as_bytes());
            raw.extend(name);
            raw.extend(&d);
            raw.resize(max_next(raw.len()), 0);
        }
        let stream = super::super::lzari::compress(&raw);
        let mut m = vec![0u8; MAX_HEADER - 4];
        m[..12].copy_from_slice(MAX_MAGIC);
        m[16..24].copy_from_slice(b"BESLES-1");
        put32(&mut m, 0x50, stream.len() as u32);
        put32(&mut m, 0x54, 2);
        m.extend(stream);
        let (k, f) = open(&m, "max").unwrap();
        assert_eq!((k, f.len(), f[1].name.as_str()), (Kind::Max, 2, "BESLES-1/DATA"));
        let edited: Vec<u8> = f[1].data.iter().map(|b| b ^ 1).collect();
        let out = save(k, &m, "BESLES-1/DATA", &edited).unwrap();
        assert_eq!(open(&out, "max").unwrap().1[1].data, edited);

        // .cbs: the same files, zlib-packed and RC4-encrypted.
        let mut raw = vec![];
        for (n, d) in [("icon.sys", vec![1u8; 100]), ("DATA", vec![5u8; 600])] {
            let mut e = vec![0u8; CBS_ENTRY];
            put32(&mut e, 16, d.len() as u32);
            e[32..32 + n.len()].copy_from_slice(n.as_bytes());
            raw.extend(e);
            raw.extend(d);
        }
        let mut enc = flate2::write::ZlibEncoder::new(vec![], flate2::Compression::default());
        std::io::Write::write_all(&mut enc, &raw).unwrap();
        let mut z = enc.finish().unwrap();
        rc4(&mut z);
        let mut c = vec![0u8; CBS_HEADER];
        c[..4].copy_from_slice(CBS_MAGIC);
        put32(&mut c, 8, CBS_HEADER as u32);
        c[0x14..0x1C].copy_from_slice(b"BASLUS-2");
        c.extend(z);
        let (k, f) = open(&c, "cbs").unwrap();
        assert_eq!((k, f[1].data.len()), (Kind::Cbs, 600));
        let out = save(k, &c, &f[1].name, &[6; 700]).unwrap();
        assert_eq!(open(&out, "cbs").unwrap().1[1].data, vec![6; 700]);
    }

    #[test]
    fn mcs_and_psv() {
        let mut m = vec![0u8; 0x80];
        m[0] = 0x51;
        put32(&mut m, 4, 0x2000);
        m[0x0A..0x16].copy_from_slice(b"BESLES-00000");
        m[0x7F] = m[..0x7F].iter().fold(0, |a, b| a ^ b);
        let mut data = vec![0u8; 0x2000];
        data[..2].copy_from_slice(b"SC");
        m.extend(&data);
        let (k, f) = open(&m, "mcs").unwrap();
        assert_eq!((k, f[0].name.as_str()), (Kind::Mcs, "BESLES-00000"));
        data[100] = 9;
        assert_eq!(open(&save(k, &m, &f[0].name, &data).unwrap(), "").unwrap().1[0].data, data);

        // A PS1 .psv: signing it again keeps a signature that checks.
        let mut p = vec![0u8; 0x84];
        p[..4].copy_from_slice(PSV_MAGIC);
        p[8..0x1C].copy_from_slice(b"www.bucanero.com.ar\0");
        p[0x38] = 0x14;
        p[0x3C] = 1;
        p[0x64..0x70].copy_from_slice(b"BESLES-00000");
        p.extend(&data);
        let (k, f) = open(&p, "psv").unwrap();
        let out = save(k, &p, &f[0].name, &data).unwrap();
        assert_eq!(&out[0x1C..0x30], &psv_signature(&out).unwrap());
        assert_ne!(&out[0x1C..0x30], &[0; 20]);
    }
}

#[cfg(test)]
mod samples {
    /// Real saves from a folder (SAVE_SAMPLES): each reads, packs back unchanged to the same
    /// files, and LZARI repacks `.max` streams byte for byte.
    #[test]
    #[ignore]
    fn real_saves() {
        let dir = std::env::var("SAVE_SAMPLES").expect("SAVE_SAMPLES");
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            let ext = p.extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
            let Ok(d) = std::fs::read(&p) else { continue };
            let Some((kind, files)) = super::open(&d, &ext) else { continue };
            if matches!(kind, super::Kind::Psv1 | super::Kind::Psv2) {
                println!("{:?}: signature checks: {}", p.file_name().unwrap(), super::psv_signature(&d).unwrap() == d[0x1C..0x30]);
            }
            for f in &files {
                let out = super::save(kind, &d, &f.name, &f.data).unwrap();
                let (_, again) = super::open(&out, &ext).unwrap();
                assert!(again.iter().zip(&files).all(|(a, b)| a.name == b.name && a.data == b.data), "{p:?}");
                if matches!(kind, super::Kind::Psv1 | super::Kind::Psv2) {
                    // Signed again: only the signature may differ (when the original's was stale).
                    assert!(out[..0x1C] == d[..0x1C] && out[0x30..] == d[0x30..], "{p:?} {} changed", f.name);
                } else if kind != super::Kind::Max {
                    assert!(out == d, "{p:?} {} changed", f.name);
                }
            }
            // An edit (one byte changed, then the file grown by 100 bytes where that's allowed) reads back.
            let big = files.iter().max_by_key(|f| f.data.len()).unwrap();
            let mut edited = big.data.clone();
            let mid = edited.len() / 2;
            edited[mid] ^= 0x5A;
            let mut tries = vec![edited.clone()];
            if !matches!(kind, super::Kind::Mcs | super::Kind::Psx | super::Kind::Psv1) {
                edited.extend([7u8; 100]);
                tries.push(edited);
            }
            for t in tries {
                let out = super::save(kind, &d, &big.name, &t).unwrap();
                let (_, again) = super::open(&out, &ext).unwrap();
                for f in &again {
                    assert_eq!(f.data, if f.name == big.name { t.clone() } else { files.iter().find(|g| g.name == f.name).unwrap().data.clone() }, "{p:?}");
                }
                if matches!(kind, super::Kind::Psv1 | super::Kind::Psv2) {
                    assert_eq!(super::psv_signature(&out).unwrap(), out[0x1C..0x30]);
                }
            }
            if kind == super::Kind::Max {
                let raw = super::super::lzari::decompress(&d[super::MAX_HEADER - 4..]).unwrap();
                let z = super::super::lzari::compress(&raw);
                let same = d[super::MAX_HEADER - 4..].starts_with(&z);
                println!("{:?}: {} files, LZARI repack identical: {same} ({} vs {})", p.file_name().unwrap(), files.len(), z.len(), d.len() - (super::MAX_HEADER - 4));
            } else {
                println!("{:?}: {:?}, {} files", p.file_name().unwrap(), kind, files.len());
            }
        }
    }
}
