//! Chrono Cross (PlayStation 1) saves: one 8 KiB block per memory card file. The game state is
//! stored as a run of bit-packed fields (0x5ac..0x1f21 of the block, little-endian, bit by bit);
//! the editor unpacks it to a work area behind the block, edits that, and packs it again on save,
//! then writes the block's checksum (a 32-bit sum of the first 0x1ff8 bytes, stored at 0x1ffc).
//! Offsets, the packing table and item names follow the MIT-licensed save editor in
//! RyudoSynbios/game-tools-collection (see THIRD_PARTY_NOTICES.md).

use super::fields::{Builder, Checksum, Kind};
use super::ps1::{is_save, title};
use crate::tree::TreeDoc;

pub const KIND: &str = "Chrono Cross save";

const PACK_FROM: usize = 0x5ac;
const PACK_LEN: usize = 0x1975;
const UNPACKED: usize = 0xe1f4;
/// Where the unpacked data sits in the working buffer (right behind the block).
const U: usize = 0x2000;

fn sum_ok(d: &[u8]) -> bool {
    let s: u32 = d[..0x1ff8].iter().fold(0u32, |a, b| a.wrapping_add(*b as u32));
    d[0x1ffc..0x2000] == s.to_le_bytes()
}

pub fn looks(d: &[u8]) -> bool {
    if !is_save(d, 1) {
        return false;
    }
    // "ＣＨＲＯ…" in full-width Latin letters, or the Japanese "クロノ".
    let t = &d[4..16];
    (t.starts_with(&[0x82, 0x62, 0x82, 0x67, 0x82, 0x71, 0x82, 0x6e]) || t.starts_with(&[0x83, 0x4e, 0x83, 0x8d, 0x83, 0x6d])) && sum_ok(d)
}

/// `(mode, bits, count, offset, step)`: the packing table.
const STEPS: &[(u8, u32, usize, usize, usize)] = &[
    (0, 9, 384, 53564, 2),
    (0, 7, 384, 54332, 1),
    (0, 9, 192, 54716, 2),
    (0, 7, 192, 55100, 1),
    (1, 3, 44, 56728, 48),
    (0, 8, 44, 43732, 204),
    (0, 8, 44, 43558, 204),
    (2, 9, 44, 43620, 2),
    (0, 7, 44, 43580, 204),
    (0, 7, 44, 43748, 204),
    (0, 7, 44, 43749, 204),
    (0, 7, 44, 43582, 204),
    (0, 7, 44, 43750, 204),
    (0, 7, 44, 43751, 204),
    (0, 7, 44, 43581, 204),
    (0, 7, 44, 43752, 204),
    (0, 7, 44, 43753, 204),
    (0, 7, 44, 43583, 204),
    (0, 7, 44, 43754, 204),
    (0, 7, 44, 43755, 204),
    (0, 7, 44, 43584, 204),
    (0, 7, 44, 43756, 204),
    (0, 7, 44, 43757, 204),
    (0, 7, 44, 43585, 204),
    (3, 10, 44, 56024, 0),
    (0, 10, 44, 43740, 204),
    (0, 10, 44, 43578, 204),
    (0, 10, 44, 43744, 204),
    (0, 9, 44, 43588, 204),
    (0, 9, 44, 43590, 204),
    (0, 9, 44, 43592, 204),
    (0, 9, 44, 43594, 204),
    (0, 9, 44, 43596, 204),
    (0, 9, 44, 43601, 204),
    (0, 10, 44, 43746, 204),
    (0, 7, 44, 43758, 204),
    (0, 7, 44, 43759, 204),
    (4, 11, 44, 55308, 0),
    (0, 32, 1, 57784, 1),
    (0, 32, 1, 57788, 1),
    (0, 32, 1, 57792, 1),
    (0, 32, 1, 57796, 1),
    (0, 32, 1, 0, 1),
    (0, 32, 1, 4, 1),
    (0, 32, 1, 8, 1),
    (0, 32, 1, 12, 1),
    (0, 32, 1, 57824, 1),
    (0, 32, 1, 57828, 1),
    (0, 8, 44, 43724, 204),
    (0, 16, 1, 57832, 1),
    (0, 8, 1, 43557, 1),
    (0, 8, 1, 52329, 1),
    (0, 32, 1, 57836, 1),
    (0, 32, 1, 57840, 1),
];

fn width_bytes(bits: u32) -> usize {
    if bits == 32 {
        4
    } else if bits > 8 {
        2
    } else {
        1
    }
}

struct Reader<'a> {
    d: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn take(&mut self, bits: u32) -> u32 {
        let mut v: u64 = 0;
        for i in 0..bits as usize {
            let p = self.pos + i;
            let bit = self.d.get(p >> 3).map_or(0, |b| (b >> (p & 7)) & 1);
            v |= (bit as u64) << i;
        }
        self.pos += bits as usize;
        v as u32
    }
}

struct Writer {
    d: Vec<u8>,
    pos: usize,
}

impl Writer {
    fn put(&mut self, v: u32, bits: u32) {
        for i in 0..bits as usize {
            let p = self.pos + i;
            if (p >> 3) < self.d.len() && (v >> i) & 1 == 1 {
                self.d[p >> 3] |= 1 << (p & 7);
            }
        }
        self.pos += bits as usize;
    }
}

fn get(d: &[u8], at: usize, n: usize) -> u32 {
    let mut v = 0u32;
    for i in 0..n {
        v |= (*d.get(at + i).unwrap_or(&0) as u32) << (8 * i);
    }
    v
}

fn set(d: &mut [u8], at: usize, n: usize, v: u32) {
    for i in 0..n {
        if let Some(b) = d.get_mut(at + i) {
            *b = (v >> (8 * i)) as u8;
        }
    }
}

pub fn unpack(packed: &[u8]) -> Vec<u8> {
    let mut r = Reader { d: packed, pos: 0 };
    let mut o = vec![0u8; UNPACKED];
    for &(mode, bits, count, off, step) in STEPS {
        match mode {
            0 => {
                for i in 0..count {
                    let v = r.take(bits);
                    set(&mut o, off + i * step, width_bytes(bits), v);
                }
            }
            1 => {
                for c in 0..count {
                    for j in 0..step {
                        let at = off + c * 0x18 + (j >> 1);
                        let v = r.take(bits) as u8;
                        if j & 1 == 0 {
                            o[at] = (o[at] & 0xf0) | (v & 0xf);
                        } else {
                            o[at] = (o[at] & 0x0f) | (v << 4);
                        }
                    }
                }
            }
            2 => {
                for c in 0..count {
                    for i in 0..0x30 {
                        let v = r.take(bits);
                        set(&mut o, off + c * 0xcc + i * step, width_bytes(bits), v);
                    }
                }
            }
            3 => {
                for c in 0..count {
                    for i in 0..8 {
                        let v = r.take(bits);
                        set(&mut o, off + c * 0x10 + i * 2, 2, v);
                    }
                }
            }
            _ => {
                for c in 0..count {
                    let mut at = off + c * 0x10;
                    for _ in 0..6 {
                        let ch = r.take(bits);
                        if ch & 0x700 != 0 {
                            o[at] = (((ch & 0x700) >> 8) as u8).wrapping_sub(9);
                            at += 1;
                        }
                        o[at] = ch as u8;
                        at += 1;
                    }
                    o[at] = 0;
                }
            }
        }
    }
    o
}

pub fn pack(o: &[u8]) -> Vec<u8> {
    let mut w = Writer { d: vec![0u8; PACK_LEN], pos: 0 };
    for &(mode, bits, count, off, step) in STEPS {
        match mode {
            0 => {
                for i in 0..count {
                    w.put(get(o, off + i * step, width_bytes(bits)), bits);
                }
            }
            1 => {
                for c in 0..count {
                    for j in 0..step {
                        let b = o[off + c * 0x18 + (j >> 1)];
                        w.put(if j & 1 == 0 { b & 0xf } else { b >> 4 } as u32, bits);
                    }
                }
            }
            2 => {
                for c in 0..count {
                    for i in 0..0x30 {
                        w.put(get(o, off + c * 0xcc + i * step, width_bytes(bits)), bits);
                    }
                }
            }
            3 => {
                for c in 0..count {
                    for i in 0..8 {
                        w.put(get(o, off + c * 0x10 + i * 2, 2), bits);
                    }
                }
            }
            _ => {
                for c in 0..count {
                    let mut at = off + c * 0x10;
                    for _ in 0..6 {
                        let mut ch = o[at] as u32;
                        at += 1;
                        if ch > 0xf7 {
                            ch = ((ch - 0xf7) << 8) | o[at] as u32;
                            at += 1;
                        }
                        w.put(ch, bits);
                    }
                }
            }
        }
    }
    w.d
}

/// Copies of a base stat that the game keeps next to it (offset of the base, of its copies).
const MIRRORS: &[(usize, &[(usize, usize)])] = &[
    (0x3a, &[(0xe0, 2)]),
    (0x3c, &[(0xe4, 1), (0xe5, 1)]),
    (0x3d, &[(0xe8, 1), (0xe9, 1)]),
    (0x3e, &[(0xe6, 1), (0xe7, 1)]),
    (0x3f, &[(0xea, 1), (0xeb, 1)]),
    (0x40, &[(0xec, 1), (0xed, 1)]),
    (0x41, &[(0xee, 1), (0xef, 1)]),
];

const PARTY: usize = 0xaa00;
const CHAR: usize = 0xcc;

fn repack(a: &mut Vec<u8>) {
    if a.len() < U + UNPACKED {
        return;
    }
    // Which bytes were edited: compare with what the packed run held when the save was opened.
    let before = unpack(&a[PACK_FROM..PACK_FROM + PACK_LEN]);
    let touched: Vec<usize> = (0..UNPACKED).filter(|&i| a[U + i] != before[i]).map(|i| U + i).collect();
    if touched.is_empty() {
        a.truncate(0x2000);
        return;
    }
    for &t in &touched {
        if t < U + PARTY || t >= U + PARTY + 44 * CHAR {
            continue;
        }
        let (c, rel) = ((t - U - PARTY) / CHAR, (t - U - PARTY) % CHAR);
        for (base, copies) in MIRRORS {
            let wid = if *base == 0x3a { 2 } else { 1 };
            if rel >= *base && rel < base + wid {
                let at = U + PARTY + c * CHAR;
                let v = get(a, at + base, wid);
                for (off, n) in *copies {
                    set(a, at + off, *n, v);
                }
            }
        }
    }
    let packed = pack(&a[U..U + UNPACKED]);
    a[PACK_FROM..PACK_FROM + PACK_LEN].copy_from_slice(&packed);
    a.truncate(0x2000);
    let s: u32 = a[..0x1ff8].iter().fold(0u32, |x, b| x.wrapping_add(*b as u32));
    a[0x1ffc..0x2000].copy_from_slice(&s.to_le_bytes());
}

fn name(o: &[u8], c: usize) -> String {
    let at = U + 0xd80c + c * 0x10;
    let mut s = String::new();
    for &b in &o[at..at + 12] {
        match b {
            0 => break,
            0x20 => s.push(' '),
            0x28..=0x31 => s.push((b'0' + (b - 0x28)) as char),
            0x32..=0x4b => s.push((b'A' + (b - 0x32)) as char),
            0x4c..=0x65 => s.push((b'a' + (b - 0x4c)) as char),
            _ => return String::new(),
        }
    }
    s.trim().to_string()
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Chrono Cross save".into());
    }
    let mut buf = d.to_vec();
    buf.resize(0x2000, 0);
    buf.extend_from_slice(&unpack(&d[PACK_FROM..PACK_FROM + PACK_LEN]));
    let mut b = Builder::new(&buf);
    b.info(&["Info", "Save title"], title(d));
    b.ranged(&["General", "Gold"], 0x58c, Kind::U32, 0, 999_999_999);
    let items: Vec<(String, u64)> = std::iter::once(("-".to_string(), 0u64)).chain(ITEMS.iter().map(|(i, n)| (n.to_string(), *i as u64))).collect();
    let els: Vec<(String, u64)> = std::iter::once(("-".to_string(), 0u64)).chain(ELEMENTS.iter().map(|(i, n)| (n.to_string(), *i as u64))).collect();
    for c in 0..44 {
        let n = name(&buf, c);
        let label = if n.is_empty() { format!("{:02} Character", c + 1) } else { format!("{:02} {}", c + 1, n) };
        let at = U + PARTY + c * CHAR;
        let p = |k: &'static str| -> [String; 3] { ["Party".to_string(), label.clone(), k.to_string()] };
        let nums: [(&str, usize, Kind, i64, i64); 8] = [
            ("HP", 0xdc, Kind::U16, 0, 999),
            ("Max HP", 0x3a, Kind::U16, 1, 999),
            ("Strength", 0x3c, Kind::U8, 0, 127),
            ("Resistance", 0x3d, Kind::U8, 0, 127),
            ("Accuracy", 0x3e, Kind::U8, 0, 127),
            ("Agility", 0x3f, Kind::U8, 0, 127),
            ("Magic Power", 0x40, Kind::U8, 0, 127),
            ("Magic Resistance", 0x41, Kind::U8, 0, 127),
        ];
        for (k, o, kind, lo, hi) in nums {
            let path = p(k);
            let r: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
            b.ranged(&r, at + o, kind, lo, hi);
        }
        for (k, o) in [("Weapon", 0x44), ("Armor", 0x46), ("Accessory 1", 0x48), ("Accessory 2", 0x4a), ("Accessory 3", 0x4c)] {
            let path = p(k);
            let r: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
            b.pick(&r, at + o, Kind::U16, &items);
        }
    }
    for i in 0..255 {
        let slot = format!("Slot {:03}", i + 1);
        b.pick(&["Items", &slot, "Item"], U + 0xd13c + i * 2, Kind::U16, &items);
        b.ranged(&["Items", &slot, "Quantity"], U + 0xd43c + i, Kind::U8, 0, 99);
    }
    for i in 0..192 {
        let slot = format!("Slot {:03}", i + 1);
        b.pick(&["Elements", &slot, "Element"], U + 0xd5bc + i * 2, Kind::U16, &els);
        b.ranged(&["Elements", &slot, "Quantity"], U + 0xd73c + i, Kind::U8, 0, 99);
    }
    Ok(b.finish(KIND, Checksum::Repack(repack)).with_note(crate::i18n::tx("Close the emulator first. The packed game data is built again from your edits, with a new checksum, when the save is written.")))
}

const ITEMS: &[(u16, &str)] = &[(0x1, "Bone"), (0x2, "Copper"), (0x3, "Iron"), (0x4, "Mythril"), (0x5, "Denadorite"), (0x6, "Rainbow Shell"), (0x7, "Eyeball"), (0x8, "Humour"), (0x9, "Feather"), (0xa, "Scale"), (0xb, "Fur"), (0xc, "Leather"), (0xd, "Fang"), (0xe, "Carapace"), (0xf, "Screw"), (0x10, "Seed"), (0x11, "Shiny Ember"), (0x12, "Shiny Dew"), (0x13, "Shiny Leaf"), (0x14, "Shiny Sand"), (0x15, "Shiny Salt"), (0x16, "Shiny Soot"), (0x20, "Sea Swallow"), (0x21, "Copper Swallow"), (0x22, "Steel Swallow"), (0x23, "Silver Swallow"), (0x24, "Stone Swallow"), (0x25, "Spectra Swallow"), (0x26, "Mastermune"), (0x27, "Ivory Dagger"), (0x28, "Bronze Dagger"), (0x29, "Iron Dagger"), (0x2a, "Mythril Dagger"), (0x2b, "Denadorite Dagger"), (0x2c, "Prism Dagger"), (0x2d, "Silver Staff"), (0x2e, "Stone Staff"), (0x2f, "Spectral Staff"), (0x30, "Porcelain Rod"), (0x31, "Brass Rod"), (0x32, "Iron Rod"), (0x33, "Mythril Rod"), (0x34, "Denadorite Rod"), (0x35, "Rainbow Rod"), (0x36, "Floral Rod"), (0x37, "Bone Sword"), (0x38, "Bronze Sword"), (0x39, "Steel Sword"), (0x3a, "Silver Sword"), (0x3b, "Stone Sword"), (0x3c, "Spectral Sword"), (0x3d, "Hero's Blade"), (0x3e, "Viper's Venom"), (0x3f, "Einlanzer"), (0x40, "Einlanzer"), (0x41, "Slasher"), (0x42, "Ferrous Gun"), (0x43, "Argent Gun"), (0x44, "Denadorite Gun"), (0x45, "Spectral Gun"), (0x46, "Shockwave Gun"), (0x47, "Plasma Pistol"), (0x48, "Porcelain Pick"), (0x49, "Brass Pick"), (0x4a, "Iron Pick"), (0x4b, "Mythril Pick"), (0x4c, "Pebble Pick"), (0x4d, "Prism Pick"), (0x4e, "Carrot"), (0x4f, "Betta Carotene"), (0x50, "Bone Axe"), (0x51, "Bronze Axe"), (0x52, "Iron Axe"), (0x53, "Silver Axe"), (0x54, "Stone Axe"), (0x55, "Rainbow Axe"), (0x57, "Great Hammer"), (0x58, "Master Hammer"), (0x59, "Bronze Lure"), (0x5a, "Iron Lure"), (0x5b, "Mythril Lure"), (0x5c, "Stone Lure"), (0x5d, "Prism Lure"), (0x5f, "Bronzerang"), (0x60, "Steelerang"), (0x61, "Silverang"), (0x62, "Rockerang"), (0x63, "Prismarang"), (0x65, "Bone Shot"), (0x66, "Bronze Shot"), (0x67, "Steel Shot"), (0x68, "Silver Shot"), (0x69, "Stone Shot"), (0x6a, "Prism Pellets"), (0x6b, "Private Deck"), (0x6c, "Pack of Lies"), (0x6e, "Spatula Ca20"), (0x6f, "Besom Cu29"), (0x70, "Ladle Fe26"), (0x71, "Frypan Ag47"), (0x72, "Saucepan SiO2"), (0x73, "Crystalpan C6"), (0x75, "Bone Glove"), (0x76, "Bronze Glove"), (0x77, "Iron Glove"), (0x78, "Mythril Glove"), (0x79, "Granite Glove"), (0x7a, "Spectral Glove"), (0x96, "Ivory Mail"), (0x97, "Bronze Mail"), (0x98, "Iron Mail"), (0x99, "Mythril Mail"), (0x9a, "Stone Mail"), (0x9b, "Prism Mail"), (0x9c, "Ivory Vest"), (0x9d, "Bronze Vest"), (0x9e, "Iron Vest"), (0x9f, "Mythril Vest"), (0xa0, "Stone Vest"), (0xa1, "Prism Vest"), (0xa2, "Blue Plate"), (0xa3, "Red Plate"), (0xa4, "Yellow Plate"), (0xa5, "Green Plate"), (0xa6, "Black Plate"), (0xa7, "White Plate"), (0xa8, "Feathery Dress"), (0xa9, "Scaley Dress"), (0xaa, "Carapace Dress"), (0xab, "Screwy Dress"), (0xac, "Diva Dress"), (0xad, "Wisp Cape"), (0xae, "Cloud Cape"), (0xaf, "Stardust Cape"), (0xb0, "Energizer Suit"), (0xb1, "Ghetz's Shirt"), (0xba, "Ozzie Pants"), (0xbb, "Flea Vest"), (0xbc, "Antitoxinal Cap"), (0xbd, "Poultice Cap"), (0xbe, "Plaster Cap"), (0xbf, "Antiviral Cap"), (0xc0, "Hero's Medal"), (0xc1, "Hero's Shield"), (0xc2, "Ivory Helmet"), (0xc3, "Bronze Helmet"), (0xc4, "Iron Helmet"), (0xc5, "Mythril Helmet"), (0xc6, "Stone Helmet"), (0xc7, "Prism Helmet"), (0xc8, "Power Glove"), (0xc9, "Dragoon Gauntlet"), (0xca, "Power Seal"), (0xcb, "Magic Ring"), (0xcc, "Sky Djinn Ring"), (0xcd, "Magic Seal"), (0xce, "Silver Loupe"), (0xcf, "Sight Scope"), (0xd0, "Third Eye"), (0xd1, "Knee Pad"), (0xd2, "Elbow Pad"), (0xd3, "Waist Pad"), (0xd4, "Silver Pendant"), (0xd5, "Gold Pendant"), (0xd6, "Memento Pendant"), (0xd7, "Dancing Shoes"), (0xd8, "Kung-fu Shoes"), (0xd9, "Winged Shoes"), (0xda, "Dragoon's Honor"), (0xdb, "Dragoon's Glory"), (0xdc, "Defender"), (0xdd, "Defender Plus"), (0xde, "Silver Earring"), (0xdf, "Gold Earring"), (0xe0, "Earring of Light"), (0xe1, "Earring of Hope"), (0xe2, "Stamina Ring"), (0xe3, "Stamina Belt"), (0xe4, "Resistance Ring"), (0xe5, "Resistance Belt"), (0xe6, "Sea Charm"), (0xe7, "Flame Charm"), (0xe8, "Forest Charm"), (0xe9, "Earth Charm"), (0xea, "Angel Charm"), (0xeb, "Daemon Charm"), (0xec, "Blue Brooch"), (0xed, "Red Brooch"), (0xee, "Green Brooch"), (0xef, "Yellow Brooch"), (0xf0, "White Brooch"), (0xf1, "Black Brooch"), (0xf2, "Dreamer's Scarf"), (0xf3, "Dreamer's Sash"), (0xf4, "Trashy Tiara"), (0xf5, "Golden Tiara"), (0xf6, "Profiteer Purse"), (0xf7, "Moonglasses"), (0xf8, "Sunglasses"), (0xf9, "Pendragon Sigil C"), (0xfa, "Pendragon Sigil B"), (0xfb, "Pendragon Sigil A"), (0xfc, "Forget-me-not Pot"), (0xfd, "Star Fragment"), (0xfe, "Dreamer's Sarong")];
const ELEMENTS: &[(u16, &str)] = &[(0x1, "AquaBeam"), (0x2, "AquaBall"), (0x3, "Deluge"), (0x4, "IceLance"), (0x5, "IceBlast"), (0x6, "Iceberg"), (0x7, "Fireball"), (0x8, "FirePillar"), (0x9, "Inferno"), (0xa, "MagmaBomb"), (0xb, "MagmaBurst"), (0xc, "Volcano"), (0xd, "Bushwhacker"), (0xe, "Bushbasher"), (0xf, "Carnivore"), (0x10, "AeroSaucer"), (0x11, "AeroBlaster"), (0x12, "Tornado"), (0x13, "Uplift"), (0x14, "Upheaval"), (0x15, "Earthquake"), (0x16, "ElectroJolt"), (0x17, "ElectroBolt"), (0x18, "ThundaStorm"), (0x19, "PhotonRay"), (0x1a, "PhotonBeam"), (0x1b, "HolyLight"), (0x1c, "Meteorite"), (0x1d, "MeteorShower"), (0x1e, "UltraNova"), (0x1f, "GravityBlow"), (0x20, "Gravitonne"), (0x21, "FreeFall"), (0x22, "HellSoul"), (0x23, "HellBound"), (0x24, "BlackHole"), (0x25, "☆FrogPrince"), (0x26, "☆BlueWhale"), (0x27, "☆RedWolf"), (0x28, "☆Salamander"), (0x29, "☆Sonja"), (0x2a, "☆Genie"), (0x2b, "☆Golem"), (0x2c, "☆ThundaSnake"), (0x2d, "☆Unicorn"), (0x2e, "☆Saints"), (0x2f, "☆MotherShip"), (0x30, "☆GrimReaper"), (0x31, "TurnBlue"), (0x32, "AntiRed"), (0x33, "Nimble"), (0x34, "Numble"), (0x35, "BlueField"), (0x36, "Vigora"), (0x37, "TurnRed"), (0x38, "AntiBlue"), (0x39, "Strengthen"), (0x3a, "Weaken"), (0x3b, "RedField"), (0x3c, "Ninety-Nine"), (0x3d, "TurnGreen"), (0x3e, "AntiYellow"), (0x3f, "EagleEye"), (0x40, "BatEye"), (0x41, "GreenField"), (0x42, "InfoScope"), (0x43, "TurnYellow"), (0x44, "AntiGreen"), (0x45, "HiRes"), (0x46, "LoRes"), (0x47, "YellowField"), (0x48, "PhysNegate"), (0x49, "TurnWhite"), (0x4a, "AntiBlack"), (0x4b, "StrongMinded"), (0x4c, "WeakMinded"), (0x4d, "Magnify"), (0x4e, "MagNegate"), (0x4f, "TurnBlack"), (0x50, "AntiWhite"), (0x51, "Genius"), (0x52, "Imbecile"), (0x53, "Diminish"), (0x54, "SealAll"), (0x55, "Cure"), (0x56, "CurePlus"), (0x57, "CureAll"), (0x58, "Heal"), (0x59, "HealAll"), (0x5a, "HealPlus"), (0x5b, "Purify"), (0x5c, "RecoverAll"), (0x5d, "HolyHealing"), (0x5e, "Revive"), (0x5f, "FullRevival"), (0x60, "Revenge"), (0x61, "Medicine"), (0x62, "Ointment"), (0x63, "Recharge"), (0x64, "Antidote"), (0x65, "Brace"), (0x66, "WhiteOut"), (0x67, "Panacea"), (0x68, "BlackOut"), (0x69, "Tablet"), (0x6a, "Capsule"), (0x6b, "Nostrum"), (0x6c, "Inferno"), (0x6d, "Volcano"), (0x6e, "RedWolf"), (0x6f, "Deluge"), (0x70, "Iceberg"), (0x71, "FrogPrince"), (0x72, "Earthquake"), (0x73, "ThundaStorm"), (0x74, "Golem"), (0x75, "Carnivore"), (0x76, "Tornado"), (0x77, "Sonja"), (0x78, "FreeFall"), (0x79, "BlackHole"), (0x7a, "MotherShip"), (0x7b, "HolyLight"), (0x7c, "UltraNova"), (0x7d, "Unicorn"), (0x7e, "ChronoCross"), (0x88, "Z-Slash"), (0x89, "DeltaForce"), (0x8a, "DraggyRider"), (0x8b, "VitalForce"), (0x8c, "TossedSalad"), (0x8d, "Flamenco"), (0x8e, "DoubleTake"), (0x8f, "PitchBlack"), (0x90, "SwordStorm"), (0x91, "DragonSpike"), (0x92, "WindSlash"), (0x93, "DashSlash"), (0x94, "TheStare"), (0x95, "MaxDefense"), (0x96, "GlideHook"), (0x97, "FeralCats"), (0x98, "ForeverZero"), (0x99, "Dash&Slash"), (0x9a, "SonicSword"), (0x9b, "Dive&Drive"), (0x9c, "GlideHook (Lynx)"), (0x9d, "FeralCats (Lynx)"), (0x9e, "ForeverZero (Lynx)"), (0x9f, "X-Strike"), (0xa0, "Dash&Slash (Serge)"), (0xa1, "Luminaire (Serge)"), (0xa2, "FlyingArrow (Serge)"), (0xa3, "Pilfer (Kid)"), (0xa4, "RedPin (Kid)"), (0xa5, "HotShot (Kid)"), (0xa6, "WandaIn (Guile)"), (0xa7, "WandaSwords (Guile)"), (0xa8, "LightninRod (Guile)"), (0xa9, "SpiralRay (Norris)"), (0xaa, "SunShower (Norris)"), (0xab, "TopShot (Norris)"), (0xac, "GrandFinale (Nikki)"), (0xad, "ChillOut (Nikki)"), (0xae, "LimeLight (Nikki)"), (0xaf, "G-Force (Viper)"), (0xb0, "AirForce (Viper)"), (0xb1, "FlagBearer (Viper)"), (0xb2, "SnakeEyes (Riddel)"), (0xb3, "SnakeSkin (Riddel)"), (0xb4, "SnakeFangs (Riddel)"), (0xb5, "DragonRider (Karsh)"), (0xb6, "AxialAxe (Karsh)"), (0xb7, "Axiomatic (Karsh)"), (0xb8, "DragonRider (Zoah)"), (0xb9, "Gyronimo (Zoah)"), (0xba, "Toss&Spike (Zoah)"), (0xbb, "Cat'sCradle (Marcy)"), (0xbc, "StringPhone (Marcy)"), (0xbd, "WebSurfer (Marcy)"), (0xbe, "HeadButt (Korcha)"), (0xbf, "Hook&Sinker (Korcha)"), (0xc0, "BigCatch (Korcha)"), (0xc1, "Pin-UpGirl (Luccia)"), (0xc2, "Mix&Match (Luccia)"), (0xc3, "TestAmeba (Luccia)"), (0xc4, "K9-Ball (Poshul)"), (0xc5, "DoggyDunnit (Poshul)"), (0xc6, "Unleashed (Poshul)"), (0xc7, "Raz-Star (Razzly)"), (0xc8, "Raz-Heart (Razzly)"), (0xc9, "Raz-Flower (Razzly)"), (0xca, "HammerBlow (Zappa)"), (0xcb, "HammerThrow (Zappa)"), (0xcc, "BallsOfIron (Zappa)"), (0xcd, "SpiceOfLife (Orcha)"), (0xce, "MysteryMenu (Orcha)"), (0xcf, "DinnerGuest (Orcha)"), (0xd0, "LongShot (Radius)"), (0xd1, "QuickDraw (Radius)"), (0xd2, "VitalEnergy (Radius)"), (0xd3, "Pillage (Fargo)"), (0xd4, "CannonBalls (Fargo)"), (0xd5, "Invincible (Fargo)"), (0xd6, "BottomsUp (Macha)"), (0xd7, "Folding (Macha)"), (0xd8, "DirtyDishes (Macha)"), (0xd9, "Dash-Gash (Glenn)"), (0xda, "SonicSword (Glenn)"), (0xdb, "Dive-Drive (Glenn)"), (0xdc, "MaidenHand (Leena)"), (0xdd, "MaidenHeart (Leena)"), (0xde, "MaidenFaith (Leena)"), (0xdf, "HeadBopper (Miki)"), (0xe0, "SexyWink (Miki)"), (0xe1, "DanceOnAir (Miki)"), (0xe2, "MoonBeams (Harle)"), (0xe3, "MoonShine (Harle)"), (0xe4, "Lunfairetic (Harle)"), (0xe5, "BeatIt (Janice)"), (0xe6, "24Carrots (Janice)"), (0xe7, "What'sUpDoc (Janice)"), (0xe8, "CoughDrop (Draggy)"), (0xe9, "CoughMix (Draggy)"), (0xea, "BigBreath (Draggy)"), (0xeb, "StarLight (Starky)"), (0xec, "StarBurst (Starky)"), (0xed, "StarStruck (Starky)"), (0xee, "Doppelgang (Sprigg)"), (0xef, "VoodooDance (Mojo)"), (0xf0, "CartWheel (Mojo)"), (0xf1, "HoodooGuroo (Mojo)"), (0xf2, "VegeChopper (Turnip)"), (0xf3, "VegeMight (Turnip)"), (0xf4, "VegOut (Turnip)"), (0xf5, "PopPopPop (NeoFio)"), (0xf6, "SlurpSlurp (NeoFio)"), (0xf7, "BamBamBam (NeoFio)"), (0xf8, "Clothesline (Greco)"), (0xf9, "Flip-flop (Greco)"), (0xfa, "GraveDigger (Greco)"), (0xfb, "JugglerVein (Skelly)"), (0xfc, "BalloonLoan (Skelly)"), (0xfd, "OnARoll (Skelly)"), (0xfe, "LumberJack (Funguy)"), (0xff, "SporeCloud (Funguy)"), (0x100, "Myconoids (Funguy)"), (0x101, "WaterBreath (Irenes)"), (0x102, "MerMelody (Irenes)"), (0x103, "SirenSong (Irenes)"), (0x104, "Snatch (Mel)"), (0x105, "Doodle (Mel)"), (0x106, "Tantfrum (Mel)"), (0x107, "RockThrow (Leah)"), (0x108, "TailSpin (Leah)"), (0x109, "TripleKick (Leah)"), (0x10a, "JumpThrow (Van)"), (0x10b, "WetPaint (Van)"), (0x10c, "PiggyBoink (Van)"), (0x10d, "BigDeal (Sneff)"), (0x10e, "HPShuffle (Sneff)"), (0x10f, "SwordTrick (Sneff)"), (0x110, "DireaShadow (Steena)"), (0x111, "HydraShadow (Steena)"), (0x112, "GaraiShadow (Steena)"), (0x113, "HighFive (Doc)"), (0x114, "Gnarly (Doc)"), (0x115, "HangTen (Doc)"), (0x116, "RocketFist (Grobyc) "), (0x117, "HairCutter (Grobyc)"), (0x118, "StrongArm (Grobyc)"), (0x119, "Medalsome (Pierre)"), (0x11a, "FoiledAgain (Pierre)"), (0x11b, "SlapOfCyrus (Pierre)"), (0x11c, "Multipunch (Orlha)"), (0x11d, "PunchDrunk (Orlha)"), (0x11e, "SisterHoods (Orlha)"), (0x11f, "Pounce (Pip: Base)"), (0x120, "Soothe (Pip: Base)"), (0x121, "Pounce (Pip: Base)"), (0x122, "Soothe (Pip: Base)"), (0x123, "Pounce (Pip: Base)"), (0x124, "Soothe (Pip: Base)"), (0x125, "Pounce (Pip: Arch Devil)"), (0x126, "Soothe (Pip: Arch Devil)"), (0x127, "Hell'sFury (Pip: Arch Devil)"), (0x128, "Pounce (Pip: Holy Beast)"), (0x129, "Soothe (Pip: Holy Beast)"), (0x12a, "CanonCannon (Pip: Holy Beast)"), (0x12b, "Pounce (Pip: Arch Angel)"), (0x12c, "Soothe (Pip: Arch Angel)"), (0x12d, "HeavenCalls (Pip: Arch Angel)")];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_unpack_round_trips() {
        let mut p = vec![0u8; PACK_LEN];
        for (i, b) in p.iter_mut().enumerate() {
            *b = (i * 31 % 251) as u8;
        }
        let u = unpack(&p);
        let q = pack(&u);
        // Fields cover the whole packed run, so unpacking what was packed gives the same data.
        assert_eq!(unpack(&q), u);
    }
}
