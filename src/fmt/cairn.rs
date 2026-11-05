//! Cairn (The Game Bakers) saves, after the pack and gzip layers. The save starts with the
//! 💿 marker; its run statistics follow the name of the current route fragment (`FRAG_<ticks>_<GUID>`;
//! newer saves add a zero byte): play
//! time in seconds and three more 64-bit totals, then 32-bit counters — deaths, then falls.
//! Worked out from four real slots against the game's own slot cards (play time to the
//! second, falls, deaths; deaths stay 0 in a Free solo run).

pub const MARKER: [u8; 4] = [0xF0, 0x9F, 0x92, 0xBF];

/// Where the statistics start (the play time), if this is a Cairn save.
pub fn stats_at(d: &[u8]) -> Option<usize> {
    if !d.starts_with(&MARKER) {
        return None;
    }
    let i64_at = |o: usize| d.get(o..o + 8).map(|b| u64::from_le_bytes(b.try_into().unwrap()));
    let is_guid = |s: &[u8]| {
        s.len() == 36
            && s.iter().enumerate().all(|(i, &c)| if [8, 13, 18, 23].contains(&i) { c == b'-' } else { c.is_ascii_hexdigit() })
    };
    let ok = |at: usize| {
        let (t, a, b, c) = (i64_at(at), i64_at(at + 8), i64_at(at + 16), i64_at(at + 24));
        matches!((t, a, b, c), (Some(t), Some(a), Some(b), Some(c)) if t > 0 && t < 100_000_000 && a <= t && b >> 32 == 0 && c >> 32 == 0)
    };
    // Newer saves (format 29 at byte 4; 24 before) have one zero byte after the name.
    let gap = usize::from(d[4] >= 29);
    (36..d.len().saturating_sub(48)).filter(|&p| is_guid(&d[p - 36..p]) && (gap == 0 || d[p] == 0)).map(|p| p + gap).find(|&at| ok(at))
}

/// (label, offset from the start of the statistics, byte width)
pub const FIELDS: [(&str, usize, usize); 3] = [("Play time (seconds)", 0, 4), ("Deaths", 32, 4), ("Falls", 36, 4)];

#[cfg(test)]
mod tests {
    #[test]
    fn finds_the_statistics() {
        let mut d = super::MARKER.to_vec();
        d.push(29);
        d.extend([0u8; 19]);
        d.extend(b"3d2a4a96-8d2b-4b11-ace3-0a69b2f1c2d3\0");
        let at = d.len();
        for v in [2916u64, 2607, 35337, 71917] {
            d.extend(v.to_le_bytes());
        }
        d.extend([0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0]);
        d.extend([0u8; 64]);
        assert_eq!(super::stats_at(&d), Some(at));
    }
}

#[cfg(test)]
mod real {
    /// Research aid: SAVE_EDITOR_CAIRN = an unpacked Cairn save.
    #[test]
    #[ignore]
    fn real_save() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_CAIRN").unwrap()).unwrap();
        let at = super::stats_at(&d).expect("statistics");
        for (l, o, _) in super::FIELDS {
            println!("{l}: {}", u32::from_le_bytes(d[at + o..at + o + 4].try_into().unwrap()));
        }
    }
}

#[cfg(test)]
mod real_pack {
    /// Research aid: SAVE_EDITOR_CAIRN_PACK = a SLOT_x.CPK; sets Falls to 3 and reads it back.
    #[test]
    #[ignore]
    fn edit_falls() {
        use crate::formats::{decode, encode, Content};
        let raw = std::fs::read(std::env::var("SAVE_EDITOR_CAIRN_PACK").unwrap()).unwrap();
        let mut dec = decode(&raw);
        let Content::Binary(d) = &mut dec.content else { panic!("not binary") };
        let at = super::stats_at(d).unwrap() + 36;
        d[at..at + 4].copy_from_slice(&3u32.to_le_bytes());
        let out = encode(&dec.layers, &dec.content).unwrap();
        assert_eq!(out.len(), raw.len());
        let back = decode(&out);
        let Content::Binary(b) = &back.content else { panic!() };
        let at = super::stats_at(b).unwrap() + 36;
        assert_eq!(u32::from_le_bytes(b[at..at + 4].try_into().unwrap()), 3);
        println!("falls now 3; pack still {} bytes", out.len());
    }
}
