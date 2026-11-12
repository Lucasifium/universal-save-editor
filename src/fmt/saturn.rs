//! Sega Saturn saves.
//!
//! Single saves exported by PC and flash-cart tools: Pseudo Saturn Kai `.BUP` (a 64-byte `Vmem`
//! header and the data), SAROO `.SRO` (`SSAVERAW`, CRC-32 over the header and data), SS Backup RAM
//! Parser `.XML` (base64 data), Save Game Manager `.B64` and PC Comms Link `.CMS` (the save's own
//! backup RAM blocks).
//!
//! Whole images: the console's 32 KB backup RAM as emulators store it (Mednafen and Beetle `.bkr`,
//! raw; Yabause and Kronos `bkram.bin`, every byte on an odd address), SAROO `SS_SAVE.BIN` (one
//! 64 KB slot per game) and SAROO `SS_MEMS.BIN`. A save in an image is read through the list of
//! file offsets its bytes live at, and written back to the same offsets, so the image is changed
//! only where the save's data is.
//!
//! Layouts follow saturn-save-tools (Damian Parrino) and SAROO's savetool (tpunix), both GPL-3.0.

use base64::Engine;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Bup,
    Sro,
    Xml,
    B64,
    Cms,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Bup => "Saturn save (.bup, Pseudo Saturn Kai)",
            Kind::Sro => "Saturn save (.sro, SAROO)",
            Kind::Xml => "Saturn save (.xml, SS Backup RAM Parser)",
            Kind::B64 => "Saturn save (.b64, Save Game Manager)",
            Kind::Cms => "Saturn save (.cms, PC Comms Link)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Image {
    /// Internal backup RAM; `true` when every byte sits on an odd address.
    BackupRam(bool),
    Saroo,
    Mems,
}

impl Image {
    pub fn label(self) -> &'static str {
        match self {
            Image::BackupRam(_) => "Saturn backup RAM",
            Image::Saroo => "SAROO save file",
            Image::Mems => "SAROO SaroMems file",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Save {
    pub name: String,
    /// File offset of every byte of the save's data, in order.
    pub at: Vec<usize>,
}

fn be16(d: &[u8], o: usize) -> Option<usize> {
    Some(u16::from_be_bytes(d.get(o..o + 2)?.try_into().ok()?) as usize)
}
fn be32(d: &[u8], o: usize) -> Option<usize> {
    Some(u32::from_be_bytes(d.get(o..o + 4)?.try_into().ok()?) as usize)
}
fn name(d: &[u8]) -> String {
    let end = d.iter().position(|&b| b == 0).unwrap_or(d.len());
    d[..end].iter().map(|&b| if (0x20..0x7f).contains(&b) { b as char } else { '_' }).collect::<String>().trim_end().to_string()
}

fn crc32(d: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in d {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB8_8320 & (crc & 1).wrapping_neg());
        }
    }
    !crc
}

const XML_HEAD: &[u8] = b"<?xml";

fn b64() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

fn tag<'a>(s: &'a str, t: &str) -> Option<(usize, &'a str)> {
    let open = format!("<{t}>");
    let start = s.find(&open)? + open.len();
    let end = start + s[start..].find(&format!("</{t}>"))?;
    Some((start, &s[start..end]))
}

/// Opens a single exported save. Returns its kind, the save's name and its data.
pub fn open(d: &[u8], ext: &str) -> Option<(Kind, String, Vec<u8>)> {
    if d.len() >= 0x40 && &d[..8] == b"SSAVERAW" {
        let size = be32(d, 0x1c)?;
        let data = d.get(0x40..0x40 + size)?.to_vec();
        return Some((Kind::Sro, name(&d[0x10..0x1b]), data));
    }
    if d.len() >= 0x40 && &d[..4] == b"Vmem" && (ext == "bup" || d.len() == 0x40 + be32(d, 0x2c)?) {
        let size = be32(d, 0x2c)?;
        let data = d.get(0x40..0x40 + size)?.to_vec();
        return Some((Kind::Bup, name(&d[0x10..0x1b]), data));
    }
    if d.starts_with(XML_HEAD) {
        let s = std::str::from_utf8(d).ok().or_else(|| None)?;
        if !s.contains("<Sega_Saturn_Save>") {
            return None;
        }
        let (_, body) = tag(s, "data")?;
        let data = b64().decode(body.split_whitespace().collect::<String>()).ok()?;
        let size: usize = tag(s, "size")?.1.trim().parse().ok()?;
        if size != data.len() {
            return None;
        }
        let n = tag(s, "name").map(|t| t.1.to_string()).unwrap_or_default();
        return Some((Kind::Xml, n, data));
    }
    if ext == "b64" && d.len() > 56 && d[54] == b'=' && d[55] == b'=' {
        let nl = d.iter().position(|&b| b == b'\n')?;
        let head = b64().decode(d[..56].to_vec()).ok()?;
        if head.len() != 40 {
            return None;
        }
        let body: Vec<u8> = d[nl + 1..].iter().copied().filter(|b| !b.is_ascii_whitespace()).collect();
        let data = b64().decode(body).ok()?;
        if be32(&head, 32)? != data.len() {
            return None;
        }
        return Some((Kind::B64, name(&head[..12]), data));
    }
    if ext == "cms" && d.len() >= 0x40 && d.len() % 0x40 == 0 && d[..4] == [0x80, 0, 0, 0] {
        let at = cms_positions(d)?;
        return Some((Kind::Cms, name(&d[4..15]), at.iter().map(|&p| d[p]).collect()));
    }
    None
}

/// File offsets of a PC Comms Link save's data: the save's backup RAM blocks one after another,
/// each starting with a 4-byte block header, the block list (ending in 0) first.
fn cms_positions(d: &[u8]) -> Option<Vec<usize>> {
    let size = be32(d, 0x1e)?;
    let stream: Vec<usize> = (0..d.len() / 0x40).flat_map(|b| (b * 0x40 + 4)..(b + 1) * 0x40).collect();
    // Stream index 0 is file offset 4; the block list starts at file offset 0x22.
    // A save that fits in its first block still has the empty list's 0.
    let mut i = 0x1e;
    loop {
        let a = *stream.get(i)?;
        let b = *stream.get(i + 1)?;
        i += 2;
        if d[a] == 0 && d[b] == 0 {
            break;
        }
    }
    let at = stream.get(i..i + size)?.to_vec();
    Some(at)
}

/// Packs edited data back into a single save in its own format.
pub fn save(kind: Kind, original: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
    let (_, _, old) = open(original, match kind {
        Kind::B64 => "b64",
        Kind::Cms => "cms",
        Kind::Bup => "bup",
        _ => "",
    })
    .ok_or("the save no longer reads")?;
    if old == data {
        return Ok(original.to_vec());
    }
    let size32 = (data.len() as u32).to_be_bytes();
    match kind {
        Kind::Bup | Kind::Sro => {
            let (size_at, crc) = if kind == Kind::Sro { (0x1c, be32(original, 0x0c).unwrap_or(0) != 0) } else { (0x2c, false) };
            let mut out = original[..0x40].to_vec();
            out[size_at..size_at + 4].copy_from_slice(&size32);
            out.extend_from_slice(data);
            out.extend_from_slice(&original[(0x40 + old.len()).min(original.len())..]);
            if crc {
                let c = crc32(&out[0x10..0x40 + data.len()]);
                out[0x0c..0x10].copy_from_slice(&c.to_be_bytes());
            }
            Ok(out)
        }
        Kind::Xml => {
            let s = std::str::from_utf8(original).map_err(|_| "the save is no longer text")?;
            let (at, body) = tag(s, "data").ok_or("the save has no data")?;
            let mut out = format!("{}{}{}", &s[..at], b64().encode(data), &s[at + body.len()..]);
            if data.len() != old.len() {
                let (at, body) = tag(&out, "size").ok_or("the save has no size")?;
                out = format!("{}{}{}", &out[..at], data.len(), &out[at + body.len()..]);
            }
            Ok(out.into_bytes())
        }
        Kind::B64 => {
            let nl = original.iter().position(|&b| b == b'\n').ok_or("bad save")?;
            let crlf = nl > 0 && original[nl - 1] == b'\r';
            let eol: &[u8] = if crlf { b"\r\n" } else { b"\n" };
            let rest = &original[nl + 1..];
            let lines: Vec<&[u8]> = rest.split(|&b| b == b'\n').map(|l| l.strip_suffix(b"\r").unwrap_or(l)).filter(|l| !l.is_empty()).collect();
            let width = if lines.len() > 1 { lines[0].len() } else { usize::MAX };
            let mut head = b64().decode(&original[..56]).map_err(|_| "bad save header")?;
            head[32..36].copy_from_slice(&size32);
            let mut out = b64().encode(&head).into_bytes();
            out.extend_from_slice(eol);
            let enc = b64().encode(data).into_bytes();
            for chunk in enc.chunks(width.max(4)) {
                out.extend_from_slice(chunk);
                out.extend_from_slice(eol);
            }
            if !rest.ends_with(b"\n") {
                out.truncate(out.len() - eol.len());
            }
            Ok(out)
        }
        Kind::Cms => {
            if data.len() != old.len() {
                return Err("a Comms Link save can't change size".into());
            }
            let mut out = original.to_vec();
            for (p, &b) in cms_positions(original).ok_or("the save no longer reads")?.iter().zip(data) {
                out[*p] = b;
            }
            Ok(out)
        }
    }
}

/// Lists the saves in a whole backup memory image.
pub fn image(d: &[u8]) -> Option<(Image, Vec<Save>)> {
    let kind = if d.len() == 0x10000 && d[1] == b'B' && d[3] == b'a' && d[5] == b'c' && d[7] == b'k' {
        let s: Vec<u8> = (0..16).map(|i| d[i * 2 + 1]).collect();
        if s != b"BackUpRam Format" {
            return None;
        }
        Image::BackupRam(true)
    } else if d.len() == 0x8000 && d.starts_with(b"BackUpRam Format") {
        Image::BackupRam(false)
    } else if d.len() >= 0x20000 && d.len() % 0x10000 == 0 && d.starts_with(b"Saroo Save File") {
        Image::Saroo
    } else if d.len() >= 0x400 * 8 && d.starts_with(b"SaroMems") {
        Image::Mems
    } else {
        return None;
    };
    let mut saves = match kind {
        Image::BackupRam(odd) => backup_ram(d, odd)?,
        Image::Saroo => saroo(d)?,
        Image::Mems => mems(d)?,
    };
    // Names must tell saves apart.
    let mut seen = std::collections::HashMap::<String, usize>::new();
    for s in &mut saves {
        let n = seen.entry(s.name.clone()).or_insert(0);
        *n += 1;
        if *n > 1 {
            s.name = format!("{} ({})", s.name, n);
        }
    }
    Some((kind, saves))
}

pub fn read(d: &[u8], s: &Save) -> Vec<u8> {
    s.at.iter().map(|&p| d[p]).collect()
}

pub fn write(d: &[u8], s: &Save, data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() != s.at.len() {
        return Err(format!("this save must stay {} bytes; it lives in fixed blocks of the memory image", s.at.len()));
    }
    let mut out = d.to_vec();
    for (&p, &b) in s.at.iter().zip(data) {
        out[p] = b;
    }
    Ok(out)
}

fn backup_ram(d: &[u8], odd: bool) -> Option<Vec<Save>> {
    const BS: usize = 64;
    let m: Vec<u8> = if odd { (0..0x8000).map(|i| d[i * 2 + 1]).collect() } else { d.to_vec() };
    let file = |p: usize| if odd { p * 2 + 1 } else { p };
    let mut out = vec![];
    for start in 2..512 {
        let bp0 = start * BS;
        if be32(&m, bp0)? != 0x8000_0000 {
            continue;
        }
        let dsize = be32(&m, bp0 + 0x1e)?;
        if dsize > 0x8000 {
            return None;
        }
        let mut list = vec![start];
        // The block list (ending in 0, empty for a one-block save) runs on into the blocks it
        // names, and the data follows it.
        let (mut nbp, mut dp) = (1usize, 0x22usize);
        {
            let mut bp = bp0;
            loop {
                let b = be16(&m, bp + dp)?;
                if b == 0 {
                    break;
                }
                if !(2..512).contains(&b) || list.len() > 512 {
                    return None;
                }
                list.push(b);
                dp += 2;
                if dp == BS {
                    bp = *list.get(nbp)? * BS;
                    nbp += 1;
                    dp = 4;
                }
            }
            dp += 2;
            if dp == BS {
                nbp += 1;
                dp = 4;
            }
        }
        let mut bp = *list.get(nbp - 1)? * BS;
        let mut at = Vec::with_capacity(dsize + 1);
        while at.len() < dsize {
            at.push(file(bp + dp));
            at.push(file(bp + dp + 1));
            dp += 2;
            if dp == BS && at.len() < dsize {
                bp = *list.get(nbp)? * BS;
                nbp += 1;
                dp = 4;
            }
        }
        at.truncate(dsize);
        out.push(Save { name: name(&m[bp0 + 4..bp0 + 15]), at });
    }
    Some(out)
}

fn bit(d: &[u8], base: usize, i: usize) -> bool {
    d.get(base + (i >> 3)).is_some_and(|b| (b >> (i & 7)) & 1 == 1)
}

fn saroo(d: &[u8]) -> Option<Vec<Save>> {
    let mut out = vec![];
    for slot in 1..4096 {
        let o = slot * 16;
        if o + 16 > 0x10000 || be32(d, o)? == 0 || (slot + 1) * 0x10000 > d.len() {
            break;
        }
        let game = name(&d[o..o + 16]);
        let base = slot * 0x10000;
        if &d[base..base + 8] != b"SaroSave" {
            continue;
        }
        let bs = be16(d, base + 0x0c)?;
        if bs == 0 || bs * 512 > 0x10000 {
            return None;
        }
        let mut block = be16(d, base + 0x3e)?;
        let mut guard = 0;
        while block != 0 {
            guard += 1;
            if guard > 512 || block >= 512 {
                return None;
            }
            let bp = base + block * bs;
            let dsize = be32(d, bp + 0x0c)?;
            let mut at = Vec::with_capacity(dsize);
            let mut blk = 1;
            while at.len() < dsize {
                while blk < 512 && !bit(d, bp + 0x40, blk) {
                    blk += 1;
                }
                if blk >= 512 {
                    return None;
                }
                let src = base + blk * bs;
                let n = (dsize - at.len()).min(bs);
                at.extend(src..src + n);
                blk += 1;
            }
            out.push(Save { name: format!("{}/{}", game, name(&d[bp..bp + 11])), at });
            block = be16(d, bp + 0x3e)?;
        }
    }
    Some(out)
}

fn mems(d: &[u8]) -> Option<Vec<Save>> {
    const BS: usize = 1024;
    let mut out = vec![];
    for i in 0..448 {
        let e = 1024 + i * 16;
        if *d.get(e)? == 0 {
            continue;
        }
        let bp = be16(d, e + 0x0e)? * BS;
        let dsize = be32(d, bp + 0x0c)?;
        let at: Vec<usize> = if dsize <= BS - 64 {
            (bp + 0x40..bp + 0x40 + dsize).collect()
        } else {
            let mut at = Vec::with_capacity(dsize);
            let mut list = bp + 0x40;
            while at.len() < dsize {
                let b = be16(d, list)?;
                if b == 0 || list >= bp + BS {
                    return None;
                }
                list += 2;
                let n = (dsize - at.len()).min(BS);
                at.extend(b * BS..b * BS + n);
            }
            at
        };
        if at.last().is_some_and(|&p| p >= d.len()) {
            return None;
        }
        out.push(Save { name: name(&d[bp..bp + 11]), at });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_saves_round_trip() {
        let data: Vec<u8> = (0..300u32).map(|i| (i * 7) as u8).collect();
        let mut sro = vec![0u8; 0x40];
        sro[..8].copy_from_slice(b"SSAVERAW");
        sro[0x10..0x18].copy_from_slice(b"GAME_01\0");
        sro[0x1c..0x20].copy_from_slice(&(data.len() as u32).to_be_bytes());
        sro.extend_from_slice(&data);
        let c = crc32(&sro[0x10..]);
        sro[0x0c..0x10].copy_from_slice(&c.to_be_bytes());
        let (k, n, got) = open(&sro, "sro").unwrap();
        assert_eq!((k, n.as_str(), &got), (Kind::Sro, "GAME_01", &data));
        assert_eq!(save(k, &sro, &got).unwrap(), sro);
        let mut edited = got.clone();
        edited[5] ^= 0xff;
        let out = save(k, &sro, &edited).unwrap();
        assert_eq!(be32(&out, 0x0c).unwrap() as u32, crc32(&out[0x10..]));
        assert_eq!(open(&out, "sro").unwrap().2, edited);

        let mut bup = vec![0u8; 0x40];
        bup[..4].copy_from_slice(b"Vmem");
        bup[0x2c..0x30].copy_from_slice(&(data.len() as u32).to_be_bytes());
        bup.extend_from_slice(&data);
        let (k, _, got) = open(&bup, "bup").unwrap();
        let out = save(k, &bup, &[got.clone(), vec![1, 2]].concat()).unwrap();
        assert_eq!(open(&out, "bup").unwrap().2.len(), data.len() + 2);
    }

    #[test]
    fn backup_ram_image() {
        // One save of 100 bytes in blocks 2, 3 and 4 (list in block 2, data after it).
        let mut m = vec![0u8; 0x8000];
        for i in 0..4 {
            m[i * 16..i * 16 + 16].copy_from_slice(b"BackUpRam Format");
        }
        let bp = 2 * 64;
        m[bp..bp + 4].copy_from_slice(&0x8000_0000u32.to_be_bytes());
        m[bp + 4..bp + 11].copy_from_slice(b"TESTSAV");
        m[bp + 0x1e..bp + 0x22].copy_from_slice(&100u32.to_be_bytes());
        m[bp + 0x22..bp + 0x24].copy_from_slice(&3u16.to_be_bytes());
        m[bp + 0x24..bp + 0x26].copy_from_slice(&4u16.to_be_bytes());
        // terminator at 0x26, data from 0x28: 24 bytes here, 60 in block 3, 16 in block 4.
        let data: Vec<u8> = (1..=100).collect();
        m[bp + 0x28..bp + 0x40].copy_from_slice(&data[..24]);
        m[3 * 64 + 4..4 * 64].copy_from_slice(&data[24..84]);
        m[4 * 64 + 4..4 * 64 + 20].copy_from_slice(&data[84..]);
        let (k, saves) = image(&m).unwrap();
        assert_eq!(k, Image::BackupRam(false));
        assert_eq!(saves[0].name, "TESTSAV");
        assert_eq!(read(&m, &saves[0]), data);
        // The same image with every byte on an odd address.
        let il: Vec<u8> = m.iter().flat_map(|&b| [0xff, b]).collect();
        let (k, saves) = image(&il).unwrap();
        assert_eq!(k, Image::BackupRam(true));
        assert_eq!(read(&il, &saves[0]), data);
        let mut e = data.clone();
        e[90] = 0xee;
        let out = write(&il, &saves[0], &e).unwrap();
        assert_eq!(read(&out, &saves[0]), e);
        assert_eq!(out.iter().zip(&il).filter(|(a, b)| a != b).count(), 1);
    }

    #[test]
    fn saroo_slot() {
        // Laid out the way SAROO's savetool imports a save: slot 1 formatted with 128-byte
        // blocks, the save's header in block 1, its data in blocks 2 to 4 (its own bitmap).
        let mut d = vec![0u8; 0x20000];
        d[..15].copy_from_slice(b"Saroo Save File");
        d[16..24].copy_from_slice(b"T-12345G");
        let base = 0x10000;
        d[base..base + 8].copy_from_slice(b"SaroSave");
        d[base + 0x0c..base + 0x0e].copy_from_slice(&0x80u16.to_be_bytes());
        d[base + 0x3e..base + 0x40].copy_from_slice(&1u16.to_be_bytes());
        d[base + 0x40] = 0b0001_1111;
        let bp = base + 0x80;
        d[bp..bp + 6].copy_from_slice(b"GAME_1");
        d[bp + 0x0c..bp + 0x10].copy_from_slice(&300u32.to_be_bytes());
        d[bp + 0x40] = 0b0001_1100;
        let data: Vec<u8> = (0..300u32).map(|i| (i % 251) as u8).collect();
        d[base + 0x100..base + 0x100 + 300].copy_from_slice(&data);
        let (k, saves) = image(&d).unwrap();
        assert_eq!(k, Image::Saroo);
        assert_eq!(saves[0].name, "T-12345G/GAME_1");
        assert_eq!(read(&d, &saves[0]), data);
    }

    /// `SATURN_SAMPLES=<dir> cargo test --release --lib saturn::tests::real_saves -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_saves() {
        let dir = std::env::var("SATURN_SAMPLES").expect("SATURN_SAMPLES");
        let mut files = vec![];
        let mut stack = vec![std::path::PathBuf::from(dir)];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() { stack.push(p) } else { files.push(p) }
            }
        }
        files.sort();
        let (mut ok, mut skipped) = (0, 0);
        let mut by_name = std::collections::HashMap::<String, Vec<u8>>::new();
        let mut singles = std::collections::HashMap::<String, Vec<u8>>::new();
        let mut images = vec![];
        for p in &files {
            let d = std::fs::read(p).unwrap();
            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
            if let Some((k, n, data)) = open(&d, &ext) {
                singles.insert(n, data.clone());
                assert_eq!(save(k, &d, &data).unwrap(), d, "{}", p.display());
                if k == Kind::Sro && be32(&d, 0x0c).unwrap() != 0 {
                    assert_eq!(be32(&d, 0x0c).unwrap() as u32, crc32(&d[0x10..0x40 + data.len()]), "crc {}", p.display());
                }
                if !data.is_empty() {
                    let mut e = data.clone();
                    let i = e.len() / 2;
                    e[i] ^= 0x5a;
                    let out = save(k, &d, &e).unwrap();
                    assert_eq!(open(&out, &ext).unwrap().2, e, "{}", p.display());
                }
                let stem = p.with_extension("").to_string_lossy().to_string();
                if let Some(other) = by_name.get(&stem) {
                    assert_eq!(other, &data, "BUP and SRO differ: {stem}");
                }
                by_name.insert(stem, data);
                ok += 1;
            } else if let Some((k, saves)) = image(&d) {
                for s in &saves {
                    let data = read(&d, s);
                    assert_eq!(write(&d, s, &data).unwrap(), d);
                    images.push((s.name.clone(), data));
                }
                println!("{} {:?}: {} saves", p.display(), k, saves.len());
                ok += 1;
            } else {
                skipped += 1;
            }
        }
        for (n, data) in &images {
            let same = singles.get(n).map(|s| s == data);
            println!("image save {n}: {} bytes, matches the single save: {same:?}", data.len());
            assert_ne!(same, Some(false), "{n}");
        }
        println!("opened {ok}, not Saturn {skipped}");
    }
}
