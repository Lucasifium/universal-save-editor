//! The 64-bit sum Rare used on its Nintendo 64 EEPROM saves (GoldenEye 007, Perfect Dark,
//! Banjo-Kazooie, Banjo-Tooie): a running hash of every byte, once forwards and once
//! backwards. After RyudoSynbios/game-tools-collection (MIT; see THIRD_PARTY_NOTICES.md).

fn hash(byte: u8, poly: u64, shift: u32) -> u64 {
    let h1 = poly.wrapping_add(u64::from(byte) << (shift & 0xf)) & 0x1_ffff_ffff;
    let h2 = ((h1 << 63) >> 31 | h1 >> 1) ^ ((h1 << 44) >> 32);
    ((h2 >> 20) & 0xfff) ^ h2
}

/// The two 64-bit accumulators over `bytes`.
pub fn sums(bytes: &[u8]) -> (u64, u64) {
    let (mut c1, mut poly, mut shift) = (0u64, 0x1_3108_b3c1u64, 0u32);
    for &b in bytes {
        poly = hash(b, poly, shift);
        c1 ^= poly;
        shift += 7;
    }
    let mut c2 = c1;
    for &b in bytes.iter().rev() {
        poly = hash(b, poly, shift);
        c2 ^= poly;
        shift += 3;
    }
    (c1, c2)
}

/// GoldenEye's and Perfect Dark's stored form: the low 32 bits of the first sum, then of the
/// two sums xor-ed (big-endian, eight bytes).
pub fn stored(bytes: &[u8]) -> [u8; 8] {
    let (c1, c2) = sums(bytes);
    (((c1 & 0xffff_ffff) << 32) | ((c1 ^ c2) & 0xffff_ffff)).to_be_bytes()
}
