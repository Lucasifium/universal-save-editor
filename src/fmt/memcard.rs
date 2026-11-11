//! Console memory-card images from emulators. Each save on the card opens on its own (chosen like
//! a zip entry) and is written back into the blocks it came from.
//!
//! PlayStation (`.mcr`, `.mcd`, `.mem`, `.vgs`, `.gme`): 128 KiB, sixteen 8 KiB blocks. Block 0
//! holds the card header and fifteen 128-byte directory frames (state, size, next block, file
//! name, XOR checksum); a save fills one or more of blocks 1–15, its first block starting with
//! the "SC" title frame. Layout as documented in the no$psx specifications ("Memory Card Data
//! Format"). DexDrive and VGS images carry an extra header in front of the raw card.

pub const PS1_CARD: usize = 128 * 1024;
const BLOCK: usize = 8192;
const FRAME: usize = 128;

/// Where the raw card starts in the image (after a DexDrive or VGS header), if this is one.
pub fn ps1_offset(d: &[u8]) -> Option<usize> {
    [0usize, 64, 3904].into_iter().find(|&o| {
        d.len() == o + PS1_CARD
            && d[o..].starts_with(b"MC\0")
            && (o != 64 || d.starts_with(b"VgsM"))
            && (o != 3904 || d.starts_with(b"123-456-STD"))
    })
}

/// One save on a PlayStation card: its entry name and its blocks in order.
#[derive(Clone, Debug, PartialEq)]
pub struct Ps1Save {
    pub name: String,
    pub blocks: Vec<usize>,
}

/// A save's title as the console's card menu shows it (Shift-JIS, mostly full-width letters),
/// in plain ASCII where it has one.
fn title(b: &[u8]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < b.len() && b[i] != 0 {
        let c = b[i];
        if c < 0x80 {
            out.push(c as char);
            i += 1;
            continue;
        }
        let Some(&d) = b.get(i + 1) else { break };
        i += 2;
        let ch = match (c, d) {
            (0x81, 0x40) => ' ',
            (0x81, 0x43) => ',',
            (0x81, 0x44) => '.',
            (0x81, 0x46) => ':',
            (0x81, 0x5E) => '/',
            (0x81, 0x5B) | (0x81, 0x7C) => '-',
            (0x81, 0x69) => '(',
            (0x81, 0x6A) => ')',
            (0x81, 0x6D) => '[',
            (0x81, 0x6E) => ']',
            (0x81, 0x95) => '&',
            (0x81, 0x96) => '*',
            (0x81, 0x93) => '%',
            (0x81, 0x94) => '#',
            (0x81, 0x49) => '!',
            (0x81, 0x48) => '?',
            (0x81, 0x7B) => '+',
            (0x81, 0x66) => '\'',
            (0x82, 0x4F..=0x58) => (b'0' + d - 0x4F) as char,
            (0x82, 0x60..=0x79) => (b'A' + d - 0x60) as char,
            (0x82, 0x81..=0x9A) => (b'a' + d - 0x81) as char,
            _ => continue,
        };
        out.push(ch);
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The saves on a raw PlayStation card.
pub fn ps1_saves(card: &[u8]) -> Vec<Ps1Save> {
    let mut out = vec![];
    for first in 1..16 {
        let f = &card[first * FRAME..(first + 1) * FRAME];
        if f[0] != 0x51 {
            continue;
        }
        let mut blocks = vec![first];
        let mut next = u16::from_le_bytes([f[8], f[9]]);
        while next != 0xFFFF && blocks.len() < 15 {
            let b = next as usize + 1;
            if !(1..16).contains(&b) || blocks.contains(&b) {
                break;
            }
            blocks.push(b);
            let e = &card[b * FRAME..];
            next = u16::from_le_bytes([e[8], e[9]]);
        }
        let file: String = f[10..31]
            .iter()
            .take_while(|&&c| c != 0)
            .map(|&c| c as char)
            .collect();
        let t = if card[first * BLOCK..].starts_with(b"SC") {
            title(&card[first * BLOCK + 4..first * BLOCK + 68])
        } else {
            String::new()
        };
        let name = if t.is_empty() {
            format!("{first:02} {file}")
        } else {
            format!("{first:02} {file} \u{2014} {t}")
        };
        out.push(Ps1Save { name, blocks });
    }
    out
}

/// A save's bytes (its blocks one after another, title and icon frames included).
pub fn ps1_read(card: &[u8], s: &Ps1Save) -> Vec<u8> {
    s.blocks
        .iter()
        .flat_map(|&b| card[b * BLOCK..(b + 1) * BLOCK].iter().copied())
        .collect()
}

/// The image with the save's blocks replaced by `data`, which must keep its size.
pub fn ps1_write(image: &[u8], offset: usize, s: &Ps1Save, data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() != s.blocks.len() * BLOCK {
        return Err(format!(
            "a save on a memory card keeps its size ({} bytes)",
            s.blocks.len() * BLOCK
        ));
    }
    let mut out = image.to_vec();
    for (i, &b) in s.blocks.iter().enumerate() {
        out[offset + b * BLOCK..offset + (b + 1) * BLOCK]
            .copy_from_slice(&data[i * BLOCK..(i + 1) * BLOCK]);
    }
    Ok(out)
}

/// PlayStation 2 memory card images (`.ps2` with ECC as PCSX2 writes them, or `.bin` without):
/// a small FAT file system. Layout and ECC as in mymc by Ross Ridge (public domain): a superblock
/// "Sony PS2 Memory Card Format ", 1 KiB clusters of 512-byte pages, each page followed (in
/// `.ps2` images) by 16 spare bytes holding a 3-byte Hamming code per 128 bytes.
pub struct Ps2Card {
    page: usize,
    per_cluster: usize,
    spare: usize,
    alloc: usize,
    ifc: Vec<u32>,
    root: u32,
}

/// One file in a save folder on a PS2 card: "folder/file", its first cluster and length.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ps2File {
    pub name: String,
    pub cluster: u32,
    pub len: usize,
}

fn le32(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(d[at..at + 4].try_into().unwrap())
}

fn parity(b: u8) -> u8 {
    (b.count_ones() & 1) as u8
}

/// The 3-byte Hamming code of 128 bytes.
fn ecc128(s: &[u8]) -> [u8; 3] {
    const MASKS: [u8; 7] = [0x55, 0x33, 0x0F, 0x00, 0xAA, 0xCC, 0xF0];
    let (mut cp, mut lp0, mut lp1) = (0x77u8, 0x7Fu8, 0x7Fu8);
    for (i, &b) in s.iter().enumerate() {
        cp ^= MASKS
            .iter()
            .enumerate()
            .fold(0, |m, (k, &mask)| m | parity(b & mask) << k);
        if parity(b) == 1 {
            lp0 ^= !(i as u8);
            lp1 ^= i as u8;
        }
    }
    [cp, lp0 & 0x7F, lp1]
}

impl Ps2Card {
    pub fn open(d: &[u8]) -> Option<Self> {
        if d.len() < 0x154 || !d.starts_with(b"Sony PS2 Memory Card Format ") {
            return None;
        }
        let u16at = |o: usize| u16::from_le_bytes([d[o], d[o + 1]]) as usize;
        let (page, per_cluster) = (u16at(0x28), u16at(0x2A));
        let clusters = le32(d, 0x30) as usize;
        if !(page == 512 || page == 1024) || !(1..=16).contains(&per_cluster) || clusters == 0 {
            return None;
        }
        let pages = clusters * per_cluster;
        let spare = if d.len() == pages * page {
            0
        } else if d.len() == pages * (page + page / 128 * 4) {
            page / 128 * 4
        } else {
            return None;
        };
        let ifc = (0..32).map(|i| le32(d, 0x50 + i * 4)).collect();
        Some(Self {
            page,
            per_cluster,
            spare,
            alloc: le32(d, 0x34) as usize,
            ifc,
            root: le32(d, 0x3C),
        })
    }

    fn cluster_bytes(&self) -> usize {
        self.page * self.per_cluster
    }

    /// Where each page of absolute cluster `n` starts in the image.
    fn pages(&self, n: usize) -> impl Iterator<Item = usize> + '_ {
        (0..self.per_cluster).map(move |p| (n * self.per_cluster + p) * (self.page + self.spare))
    }

    fn cluster(&self, d: &[u8], n: usize) -> Option<Vec<u8>> {
        let mut out = Vec::with_capacity(self.cluster_bytes());
        for at in self.pages(n) {
            out.extend_from_slice(d.get(at..at + self.page)?);
        }
        Some(out)
    }

    fn fat(&self, d: &[u8], n: u32) -> Option<u32> {
        let per = self.cluster_bytes() / 4;
        let (fat_index, offset) = (n as usize / per, n as usize % per);
        let indirect = self.cluster(d, *self.ifc.get(fat_index / per)? as usize)?;
        let fat_cluster = le32(&indirect, fat_index % per * 4);
        let fat = self.cluster(d, fat_cluster as usize)?;
        Some(le32(&fat, offset * 4))
    }

    /// The clusters (relative to the allocatable area) of the chain starting at `first`.
    fn chain(&self, d: &[u8], first: u32, max: usize) -> Option<Vec<u32>> {
        let mut out = vec![first];
        let mut cur = first;
        while out.len() < max {
            let next = self.fat(d, cur)?;
            if next == 0xFFFF_FFFF || next & 0x8000_0000 == 0 {
                break;
            }
            cur = next & 0x7FFF_FFFF;
            if out.contains(&cur) {
                return None;
            }
            out.push(cur);
        }
        Some(out)
    }

    fn read(&self, d: &[u8], first: u32, len: usize) -> Option<Vec<u8>> {
        let cb = self.cluster_bytes();
        let mut out = vec![];
        for c in self.chain(d, first, len.div_ceil(cb).max(1))? {
            out.extend(self.cluster(d, c as usize + self.alloc)?);
        }
        (out.len() >= len).then(|| {
            out.truncate(len);
            out
        })
    }

    /// Directory entries: (mode, length, cluster, name).
    fn dir(&self, d: &[u8], first: u32, count: usize) -> Option<Vec<(u16, usize, u32, String)>> {
        let raw = self.read(d, first, count * 512)?;
        Some(
            raw.chunks(512)
                .map(|e| {
                    let name = e[0x40..0x60]
                        .iter()
                        .take_while(|&&c| c != 0)
                        .map(|&c| c as char)
                        .collect();
                    (
                        u16::from_le_bytes([e[0], e[1]]),
                        le32(e, 4) as usize,
                        le32(e, 0x10),
                        name,
                    )
                })
                .collect(),
        )
    }

    /// Every file in every save folder.
    pub fn files(&self, d: &[u8]) -> Option<Vec<Ps2File>> {
        let first = self.dir(d, self.root, 1)?;
        let root = self.dir(d, self.root, first.first()?.1.min(4096))?;
        let mut out = vec![];
        for (mode, len, cluster, name) in root.iter().skip(2) {
            if mode & 0xA020 != 0xA020 && mode & 0x8020 != 0x8020 {
                continue;
            }
            for (m, flen, c, fname) in self.dir(d, *cluster, (*len).min(4096))?.into_iter().skip(2)
            {
                if m & 0x8010 == 0x8010 {
                    out.push(Ps2File {
                        name: format!("{name}/{fname}"),
                        cluster: c,
                        len: flen,
                    });
                }
            }
        }
        Some(out)
    }

    pub fn read_file(&self, d: &[u8], f: &Ps2File) -> Option<Vec<u8>> {
        self.read(d, f.cluster, f.len)
    }

    /// The image with `f`'s contents replaced by `data` (same length), with the ECC of every
    /// changed page written again.
    pub fn write_file(&self, d: &[u8], f: &Ps2File, data: &[u8]) -> Result<Vec<u8>, String> {
        if data.len() != f.len {
            return Err(format!(
                "a file on a PS2 memory card keeps its size ({} bytes)",
                f.len
            ));
        }
        let cb = self.cluster_bytes();
        let chain = self
            .chain(d, f.cluster, f.len.div_ceil(cb).max(1))
            .ok_or("the file's clusters can't be followed")?;
        let mut out = d.to_vec();
        for (i, c) in chain.iter().enumerate() {
            for (p, at) in self.pages(*c as usize + self.alloc).enumerate() {
                let from = i * cb + p * self.page;
                if from >= data.len() {
                    break;
                }
                let to = (from + self.page).min(data.len());
                let page = &mut out[at..at + self.page];
                if page[..to - from] == data[from..to] {
                    continue;
                }
                page[..to - from].copy_from_slice(&data[from..to]);
                if self.spare > 0 {
                    let codes: Vec<u8> = out[at..at + self.page]
                        .chunks(128)
                        .flat_map(ecc128)
                        .collect();
                    out[at + self.page..at + self.page + codes.len()].copy_from_slice(&codes);
                }
            }
        }
        Ok(out)
    }

    /// True when every page's stored ECC matches its data (erased pages and images without ECC
    /// always pass).
    #[cfg(test)]
    pub fn ecc_ok(&self, d: &[u8]) -> bool {
        self.spare == 0
            || d.chunks(self.page + self.spare).all(|p| {
                p.iter().all(|&b| b == 0xFF)
                    || p[..self.page]
                        .chunks(128)
                        .flat_map(ecc128)
                        .eq(p[self.page..self.page + self.page / 128 * 3]
                            .iter()
                            .copied())
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(state: u8, size: u32, next: u16, name: &str) -> [u8; FRAME] {
        let mut f = [0u8; FRAME];
        f[0] = state;
        f[4..8].copy_from_slice(&size.to_le_bytes());
        f[8..10].copy_from_slice(&next.to_le_bytes());
        f[10..10 + name.len()].copy_from_slice(name.as_bytes());
        f[127] = f[..127].iter().fold(0, |a, b| a ^ b);
        f
    }

    #[test]
    fn two_block_save_round_trip() {
        let mut card = vec![0u8; PS1_CARD];
        card[..2].copy_from_slice(b"MC");
        card[FRAME..2 * FRAME].copy_from_slice(&frame(0x51, 16384, 2, "BASLUS-00000TEST"));
        card[3 * FRAME..4 * FRAME].copy_from_slice(&frame(0x53, 0, 0xFFFF, ""));
        card[2 * FRAME..3 * FRAME].copy_from_slice(&frame(0xA0, 0, 0xFFFF, ""));
        card[BLOCK..BLOCK + 4].copy_from_slice(b"SC\x11\x02");
        card[BLOCK + 4..BLOCK + 10].copy_from_slice(&[0x82, 0x60, 0x81, 0x40, 0x82, 0x50]);
        card[3 * BLOCK] = 7;
        let mut image = b"VgsM".to_vec();
        image.resize(64, 0);
        image.extend(&card);
        let o = ps1_offset(&image).unwrap();
        assert_eq!(o, 64);
        let saves = ps1_saves(&image[o..]);
        assert_eq!(
            saves,
            vec![Ps1Save {
                name: "01 BASLUS-00000TEST \u{2014} A 1".into(),
                blocks: vec![1, 3]
            }]
        );
        let mut data = ps1_read(&image[o..], &saves[0]);
        assert_eq!(data[BLOCK], 7);
        data[BLOCK] = 9;
        let out = ps1_write(&image, o, &saves[0], &data).unwrap();
        assert_eq!(out[64 + 3 * BLOCK], 9);
        assert!(ps1_write(&image, o, &saves[0], &data[1..]).is_err());
    }

    /// PS2_CARD_SAMPLE = a .ps2 or .bin card image: every file reads, writes back unchanged,
    /// and an edit keeps the ECC right.
    #[test]
    #[ignore]
    fn ps2_sample_card() {
        let d = std::fs::read(std::env::var("PS2_CARD_SAMPLE").unwrap()).unwrap();
        let c = Ps2Card::open(&d).expect("a PS2 card");
        assert!(c.ecc_ok(&d));
        for f in c.files(&d).unwrap() {
            let data = c.read_file(&d, &f).unwrap();
            println!("{}: {} bytes", f.name, data.len());
            assert_eq!(c.write_file(&d, &f, &data).unwrap(), d);
            let mut e = data.clone();
            e[f.len / 2] ^= 0x5A;
            let out = c.write_file(&d, &f, &e).unwrap();
            assert!(c.ecc_ok(&out));
            assert_eq!(c.read_file(&out, &f).unwrap(), e);
            if let Ok(p) = std::env::var("PS2_CARD_OUT") {
                std::fs::write(p, &out).unwrap();
            }
        }
    }

    /// PS1_CARD_SAMPLE = a raw card image: every save opens and writes back unchanged.
    #[test]
    #[ignore]
    fn sample_card() {
        let d = std::fs::read(std::env::var("PS1_CARD_SAMPLE").unwrap()).unwrap();
        let o = ps1_offset(&d).unwrap();
        for s in ps1_saves(&d[o..]) {
            println!("{}: {} blocks", s.name, s.blocks.len());
            assert_eq!(ps1_write(&d, o, &s, &ps1_read(&d[o..], &s)).unwrap(), d);
        }
    }
}

/// DeSmuME's `.dsv`: the raw Nintendo DS save, then a footer that starts with this text and
/// ends with "|-DESMUME SAVE-|". Returns where the footer starts.
pub fn dsv_split(d: &[u8]) -> Option<usize> {
    const SNIP: &[u8] =
        b"|<--Snip above here to create a raw sav by excluding this DeSmuME savedata footer:";
    if !d.ends_with(b"|-DESMUME SAVE-|") || d.len() > 64 * 1024 * 1024 {
        return None;
    }
    let from = d.len().saturating_sub(256);
    d[from..]
        .windows(SNIP.len())
        .position(|w| w == SNIP)
        .map(|p| from + p)
        .filter(|&p| p > 0)
}

#[cfg(test)]
mod dsv_tests {
    #[test]
    fn split() {
        let mut d = vec![1u8; 512];
        d.extend_from_slice(
            b"|<--Snip above here to create a raw sav by excluding this DeSmuME savedata footer:",
        );
        d.extend_from_slice(&[0u8; 24]);
        d.extend_from_slice(b"|-DESMUME SAVE-|");
        assert_eq!(super::dsv_split(&d), Some(512));
        assert_eq!(super::dsv_split(&d[..600]), None);
    }
}
