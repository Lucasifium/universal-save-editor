//! Kagero: Deception II (PlayStation 1, USA and Europe) saves: one 8 KiB block per save. After the
//! icon header the data is stored in 128-byte pieces, each holding 127 bytes of data and a check
//! byte (the XOR of the 127 bytes, plus one); the editor reads the pieces as one run of bytes and
//! cuts them up again, with new check bytes, when it writes the save. Offsets follow the
//! MIT-licensed save editor in RyudoSynbios/game-tools-collection (see THIRD_PARTY_NOTICES.md).

use super::fields::{Builder, Checksum, Kind};
use super::ps1::{is_save, opts};
use super::retro::flag;
use crate::tree::TreeDoc;

pub const KIND: &str = "Kagero: Deception II save";

/// The 127-byte pieces of the data, one after the other, put back at offset 0x200 of a block-sized
/// buffer (so every offset below is the template's offset plus 0x200).
const BASE: usize = 0x200;

fn piece_ok(d: &[u8]) -> bool {
    (0..60).all(|i| {
        let p = &d[BASE + i * 0x80..BASE + i * 0x80 + 0x80];
        p[0x7f] == p[..0x7f].iter().fold(0u8, |a, b| a ^ b).wrapping_add(1)
    })
}

pub fn looks(d: &[u8]) -> bool {
    // "　【ＫＡＧＥＲＯ】" (the Japanese release is laid out differently and is left alone).
    is_save(d, 1) && d[4..14] == [0x81, 0x40, 0x81, 0x79, 0x82, 0x6a, 0x82, 0x60, 0x82, 0x66] && piece_ok(d)
}

fn join(d: &[u8]) -> Vec<u8> {
    let mut a = d[..BASE].to_vec();
    for i in 0..60 {
        a.extend_from_slice(&d[BASE + i * 0x80..BASE + i * 0x80 + 0x7f]);
    }
    a.resize(0x2000, 0);
    a
}

fn split(a: &mut Vec<u8>) {
    if a.len() < 0x2000 {
        return;
    }
    let data: Vec<u8> = a[BASE..BASE + 60 * 0x7f].to_vec();
    for i in 0..60 {
        let p = BASE + i * 0x80;
        a[p..p + 0x7f].copy_from_slice(&data[i * 0x7f..i * 0x7f + 0x7f]);
        a[p + 0x7f] = data[i * 0x7f..i * 0x7f + 0x7f].iter().fold(0u8, |x, b| x ^ b).wrapping_add(1);
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Kagero save".into());
    }
    let data = join(d);
    let mut b = Builder::new(&data);
    let o = BASE;
    b.ranged(&["General", "Clear count"], o + 0x69, Kind::U8, 0, 99);
    b.ranged(&["General", "Total Ark"], o + 0x45, Kind::U32, 0, 9_999_999);
    b.ranged(&["General", "Ark"], o + 0x70d, Kind::U32, 0, 9_999_999);
    b.ranged(&["General", "Slain enemies"], o + 0x691, Kind::U8, 0, 255);
    for i in 0..4 {
        flag(&mut b, &["Endings", &format!("Ending {}", i + 1)], o + 0x49, i);
    }
    let kinds = ["Ceiling", "Wall", "Floor"];
    for (idx, ty, name) in TRAPS {
        flag(&mut b, &["Unlocked traps", kinds[*ty], name], o + 0xb6d + *idx, 6);
        b.ranged(&["Trap uses", kinds[*ty], name], o + 0xbac + idx * 2, Kind::U16, 0, 9999);
    }
    b.pick(&["Options", "Sound mode"], o + 0x63, Kind::U8, &opts(&[("Stereo", 0), ("Monaural", 1)]));
    b.pick(&["Options", "Vibration"], o + 0x66, Kind::U8, &opts(&[("Off", 0), ("On", 1)]));
    b.pick(&["Options", "Replay trap scene"], o + 0x67, Kind::U8, &opts(&[("Off", 0), ("On", 1)]));
    Ok(b.finish(KIND, Checksum::Repack(split)).with_note(crate::i18n::tx("Close the emulator first. The data is cut up into its 128-byte pieces again, each with a new check byte, when the save is written.")))
}

const TRAPS: &[(usize, usize, &str)] = &[
    (0x22, 0, "Mega Rock"),
    (0x23, 0, "Spike Rock"),
    (0x24, 0, "Iron Ball"),
    (0x25, 0, "Flare Rock"),
    (0x26, 0, "Volt Rock"),
    (0x28, 0, "Slow Gas"),
    (0x27, 0, "Confuse Gas"),
    (0x29, 0, "Evil Breath"),
    (0x2a, 0, "Berserk Gas"),
    (0x2b, 0, "Heat Breath"),
    (0x2c, 0, "Vase"),
    (0x2d, 0, "Oil Vase"),
    (0x2e, 0, "Lava Vase"),
    (0x2f, 0, "Basin"),
    (0x30, 0, "Vat"),
    (0x31, 0, "Suezo"),
    (0x33, 0, "Ardebaran"),
    (0x32, 0, "Evil Stomp"),
    (0x00, 1, "Press Wall"),
    (0x01, 1, "Attack Wall"),
    (0x02, 1, "Spike Wall"),
    (0x06, 1, "Arrow Slit"),
    (0x09, 1, "Fire Arrow"),
    (0x07, 1, "Triple Arrow"),
    (0x0d, 1, "Rolling Bomb"),
    (0x08, 1, "Chain Needle"),
    (0x0a, 1, "Fire Ball"),
    (0x35, 1, "Cold Arrow"),
    (0x0b, 1, "Buzz Saw"),
    (0x0c, 1, "Laser Arrow"),
    (0x03, 1, "Magnet"),
    (0x04, 1, "Power Magnet"),
    (0x05, 1, "Volt Magnet"),
    (0x34, 1, "Evil Kick"),
    (0x14, 2, "Small Bomb"),
    (0x17, 2, "Flash Bomb"),
    (0x18, 2, "Land Mine"),
    (0x15, 2, "Blast Bomb"),
    (0x19, 2, "Quake Bomb"),
    (0x1a, 2, "Catastrophe"),
    (0x16, 2, "Hell Fire"),
    (0x1b, 2, "Bear Trap"),
    (0x1c, 2, "Heavy Claw"),
    (0x1d, 2, "Cold Claw"),
    (0x0e, 2, "Lift Floor"),
    (0x0f, 2, "Rising Floor"),
    (0x10, 2, "Spring Floor"),
    (0x11, 2, "Smash Floor"),
    (0x1e, 2, "Spark Rod"),
    (0x1f, 2, "Thunder Volt"),
    (0x20, 2, "Judgement"),
    (0x12, 2, "Magnet Floor"),
    (0x13, 2, "Vacuum Floor"),
    (0x21, 2, "Magic Sac"),
    (0x36, 2, "Evil Upper"),
];
