//! Gran Turismo (PlayStation 1, the 1997 game) saves: five 8 KiB blocks. Offsets and names
//! follow the MIT-licensed save editor in RyudoSynbios/game-tools-collection (see
//! THIRD_PARTY_NOTICES.md). A 32-bit checksum sits at 0x6da4 and covers 0x200..0x6da4.

use super::fields::{Builder, Checksum, Kind};
use super::ps1::{is_save, title};
use crate::fmt::ps1::opts;
use crate::tree::TreeDoc;

pub const KIND: &str = "Gran Turismo save";

fn sum(d: &[u8]) -> u32 {
    let (mut c1, mut c2) = (0xaaaau32, 0x3770u32);
    for &b in &d[0x200..0x6da4] {
        let mut x = b as u32;
        c1 = c1.wrapping_add(x) ^ (x << 8);
        for _ in 0..8 {
            c2 <<= 1;
            if c2 & 0x10000 != 0 {
                c2 ^= 0x11021;
            }
            c2 |= (x >> 7) & 1;
            x <<= 1;
        }
    }
    (c1 << 16) | (c2 & 0xffff)
}

pub fn looks(d: &[u8]) -> bool {
    is_save(d, 5) && title(d).len() > 0 && u32::from_le_bytes(d[0x6da4..0x6da8].try_into().unwrap()) == sum(d)
}

const LICENSES: [(&str, [&str; 8]); 3] = [
    ("B", ["Starting and stopping 1", "Starting and stopping 2", "Basics of cornering 1", "Basics of cornering 2", "Basics of cornering 3", "Basics of multiple cornering 1", "Basics of multiple cornering 2", "B-Class License final test"]),
    ("A", ["Practical Cornering 1", "Practical Cornering 2", "Practical Cornering 3", "Handling Multiple Corners 1", "Handling Multiple Corners 2", "Handling Multiple Corners 3", "Advanced Techniques", "A-Class License final test"]),
    ("IA", ["High Speed Ring time trial", "SS Route 5 time trial", "Grand Valley time trial", "Deep Forest time trial", "Autumn Ring time trial", "Trial Mountain time trial", "SS Route 11 time trial", "IA-Class License final test"]),
];

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Gran Turismo save".into());
    }
    let mut b = Builder::new(d);
    b.info(&["Info", "Save title"], title(d));
    b.ranged(&["General", "Credits"], 0x204, Kind::U32, 0, 2_000_000_000);
    b.ranged(&["General", "Days"], 0x208, Kind::U32, 0, 99_999);
    let lic = opts(&[("-", 0), ("Bronze", 1), ("Silver", 2), ("Gold", 3)]);
    for (i, (class, tests)) in LICENSES.iter().enumerate() {
        for (j, t) in tests.iter().enumerate() {
            b.pick(&["Licenses", &format!("{class}-Class License"), t], 0x2d54 + 0x700 * i + 0xe0 * j, Kind::U8, &lic);
        }
    }
    let place = opts(&[("-", 0), ("Gold", 1), ("Silver", 2), ("Bronze", 3), ("4th Place", 4), ("5th Place", 5), ("6th Place", 6)]);
    for (sec, list) in [
        ("GT League", &[("Sunday Cup", 0x426c), ("Clubman Cup", 0x426d), ("GT Cup", 0x426e), ("GT World Cup", 0x426f)][..]),
        (
            "Special Event",
            &[("Normal Cars", 0x4270), ("Tuned Cars", 0x4271), ("Lightweight Cars", 0x4272), ("FF Cars", 0x4273), ("FR Cars", 0x4274), ("4WD Cars", 0x4275), ("Megaspeed", 0x427b), ("US v Japan", 0x427c), ("UK v Japan", 0x427d), ("UK v US", 0x427e), ("Grand Valley", 0x427f), ("All-night I", 0x4280), ("All-night II", 0x4281)],
        ),
        ("Spot Race", &[("High Speed Ring", 0x4276), ("Grand Valley East", 0x4277), ("Autumn Ring-Mini", 0x4278), ("Trial Mountain", 0x4279), ("Deep Forest", 0x427a)]),
    ] {
        for (n, at) in list {
            b.pick(&["Races", sec, n], *at, Kind::U8, &place);
        }
    }
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note(crate::i18n::tx("Close the emulator first. The licence tests and race results show the medal or place you got; the save's checksum is written again. Cars and tuning parts are not edited yet.")))
}

fn fix(d: &mut Vec<u8>) {
    if d.len() >= 0x6da8 {
        let c = sum(d);
        d[0x6da4..0x6da8].copy_from_slice(&c.to_le_bytes());
    }
}
