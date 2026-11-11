//! LZARI (LZSS with arithmetic coding), used by Action Replay Max `.max` PS2 saves.
//! Ported from Haruhiko Okumura's LZARI.C (1989, "use, distribute, and modify this program
//! freely") as adapted for memory buffers by Luigi Auriemma and Wesley Castro in
//! psv-save-converter (GPL-3.0, Damian Parrino).
//!
//! The stream starts with the unpacked size as a little-endian u32.

const N: usize = 4096;
const F: usize = 60;
const THRESHOLD: usize = 2;
const NIL: usize = N;
const M: u32 = 15;
const Q1: u64 = 1 << M;
const Q2: u64 = 2 * Q1;
const Q3: u64 = 3 * Q1;
const Q4: u64 = 4 * Q1;
const MAX_CUM: u32 = (Q1 - 1) as u32;
const N_CHAR: usize = 256 - THRESHOLD + F;
/// How many bytes past the end of the input the decoder may invent before giving up.
const FLUSH_SLACK: usize = 8;

struct Model {
    char_to_sym: Vec<usize>,
    sym_to_char: Vec<usize>,
    sym_freq: Vec<u32>,
    sym_cum: Vec<u32>,
    position_cum: Vec<u32>,
}

impl Model {
    fn new() -> Self {
        let mut m = Model {
            char_to_sym: vec![0; N_CHAR],
            sym_to_char: vec![0; N_CHAR + 1],
            sym_freq: vec![0; N_CHAR + 1],
            sym_cum: vec![0; N_CHAR + 1],
            position_cum: vec![0; N + 1],
        };
        for sym in (1..=N_CHAR).rev() {
            let ch = sym - 1;
            m.char_to_sym[ch] = sym;
            m.sym_to_char[sym] = ch;
            m.sym_freq[sym] = 1;
            m.sym_cum[sym - 1] = m.sym_cum[sym] + m.sym_freq[sym];
        }
        m.sym_freq[0] = 0;
        for i in (1..=N).rev() {
            m.position_cum[i - 1] = m.position_cum[i] + (10000 / (i + 200)) as u32;
        }
        m
    }

    fn update(&mut self, sym: usize) {
        if self.sym_cum[0] >= MAX_CUM {
            let mut c = 0;
            for i in (1..=N_CHAR).rev() {
                self.sym_cum[i] = c;
                self.sym_freq[i] = (self.sym_freq[i] + 1) >> 1;
                c += self.sym_freq[i];
            }
            self.sym_cum[0] = c;
        }
        let mut i = sym;
        while self.sym_freq[i] == self.sym_freq[i - 1] {
            i -= 1;
        }
        if i < sym {
            let (ch_i, ch_sym) = (self.sym_to_char[i], self.sym_to_char[sym]);
            self.sym_to_char[i] = ch_sym;
            self.sym_to_char[sym] = ch_i;
            self.char_to_sym[ch_i] = sym;
            self.char_to_sym[ch_sym] = i;
        }
        self.sym_freq[i] += 1;
        for k in 0..i {
            self.sym_cum[k] += 1;
        }
    }
}

/// Unpacks a stream; `None` when it is empty or breaks off well before its stated size.
pub fn decompress(input: &[u8]) -> Option<Vec<u8>> {
    let size = u32::from_le_bytes(input.get(..4)?.try_into().ok()?) as usize;
    if size == 0 || size > 256 << 20 {
        return None;
    }
    let mut pos = 4;
    let (mut buffer, mut mask) = (0u32, 0u32);
    let mut past_end = 0;
    let mut get_bit = |pos: &mut usize, past_end: &mut usize| -> u64 {
        mask >>= 1;
        if mask == 0 {
            buffer = match input.get(*pos) {
                Some(&b) => b as u32,
                None => {
                    *past_end += 1;
                    0
                }
            };
            *pos += 1;
            mask = 128;
        }
        u64::from(buffer & mask != 0)
    };
    let mut m = Model::new();
    let (mut low, mut high, mut value) = (0u64, Q4, 0u64);
    for _ in 0..M + 2 {
        value = 2 * value + get_bit(&mut pos, &mut past_end);
    }
    let mut text = [b' '; N];
    let mut r = N - F;
    let mut out = Vec::with_capacity(size);
    while out.len() < size {
        if past_end > FLUSH_SLACK {
            return None;
        }
        // DecodeChar
        let range = high - low;
        if value < low || value >= Q4 || range == 0 {
            return None;
        }
        let x = (((value - low + 1) * m.sym_cum[0] as u64 - 1) / range) as u32;
        let (mut i, mut j) = (1, N_CHAR);
        while i < j {
            let k = (i + j) / 2;
            if m.sym_cum[k] > x {
                i = k + 1;
            } else {
                j = k;
            }
        }
        let sym = i;
        high = low + (range * m.sym_cum[sym - 1] as u64) / m.sym_cum[0] as u64;
        low += (range * m.sym_cum[sym] as u64) / m.sym_cum[0] as u64;
        loop {
            if low >= Q2 {
                value = value.wrapping_sub(Q2);
                low -= Q2;
                high -= Q2;
            } else if low >= Q1 && high <= Q3 {
                value = value.wrapping_sub(Q1);
                low -= Q1;
                high -= Q1;
            } else if high > Q2 {
                break;
            }
            low += low;
            high += high;
            value = 2 * value + get_bit(&mut pos, &mut past_end);
        }
        let c = m.sym_to_char[sym];
        m.update(sym);
        if c < 256 {
            out.push(c as u8);
            text[r] = c as u8;
            r = (r + 1) & (N - 1);
            continue;
        }
        // DecodePosition
        let range = high - low;
        if value < low || value >= Q4 || range == 0 {
            return None;
        }
        let x = (((value - low + 1) * m.position_cum[0] as u64 - 1) / range) as u32;
        let (mut i, mut j) = (1, N);
        while i < j {
            let k = (i + j) / 2;
            if m.position_cum[k] > x {
                i = k + 1;
            } else {
                j = k;
            }
        }
        let position = i - 1;
        high = low + (range * m.position_cum[position] as u64) / m.position_cum[0] as u64;
        low += (range * m.position_cum[position + 1] as u64) / m.position_cum[0] as u64;
        loop {
            if low >= Q2 {
                value = value.wrapping_sub(Q2);
                low -= Q2;
                high -= Q2;
            } else if low >= Q1 && high <= Q3 {
                value = value.wrapping_sub(Q1);
                low -= Q1;
                high -= Q1;
            } else if high > Q2 {
                break;
            }
            low += low;
            high += high;
            value = 2 * value + get_bit(&mut pos, &mut past_end);
        }
        let from = (r + N - position - 1) & (N - 1);
        for k in 0..c - 255 + THRESHOLD {
            let b = text[(from + k) & (N - 1)];
            out.push(b);
            text[r] = b;
            r = (r + 1) & (N - 1);
        }
    }
    out.truncate(size);
    Some(out)
}

struct Encoder {
    out: Vec<u8>,
    buffer: u8,
    mask: u8,
    low: u64,
    high: u64,
    shifts: u32,
    m: Model,
}

impl Encoder {
    fn put_bit(&mut self, bit: bool) {
        if bit {
            self.buffer |= self.mask;
        }
        self.mask >>= 1;
        if self.mask == 0 {
            self.out.push(self.buffer);
            self.buffer = 0;
            self.mask = 128;
        }
    }

    fn output(&mut self, bit: bool) {
        self.put_bit(bit);
        while self.shifts > 0 {
            self.put_bit(!bit);
            self.shifts -= 1;
        }
    }

    fn settle(&mut self) {
        loop {
            if self.high <= Q2 {
                self.output(false);
            } else if self.low >= Q2 {
                self.output(true);
                self.low -= Q2;
                self.high -= Q2;
            } else if self.low >= Q1 && self.high <= Q3 {
                self.shifts += 1;
                self.low -= Q1;
                self.high -= Q1;
            } else {
                break;
            }
            self.low += self.low;
            self.high += self.high;
        }
    }

    fn char(&mut self, ch: usize) {
        let sym = self.m.char_to_sym[ch];
        let range = self.high - self.low;
        let total = self.m.sym_cum[0] as u64;
        self.high = self.low + (range * self.m.sym_cum[sym - 1] as u64) / total;
        self.low += (range * self.m.sym_cum[sym] as u64) / total;
        self.settle();
        self.m.update(sym);
    }

    fn position(&mut self, p: usize) {
        let range = self.high - self.low;
        let total = self.m.position_cum[0] as u64;
        self.high = self.low + (range * self.m.position_cum[p] as u64) / total;
        self.low += (range * self.m.position_cum[p + 1] as u64) / total;
        self.settle();
    }
}

/// The LZSS search trees over the ring buffer.
struct Tree {
    text: Vec<u8>,
    lson: Vec<usize>,
    rson: Vec<usize>,
    dad: Vec<usize>,
    match_position: usize,
    match_length: usize,
}

impl Tree {
    fn new() -> Self {
        let mut t = Tree {
            text: vec![0; N + F - 1],
            lson: vec![NIL; N + 1],
            rson: vec![NIL; N + 257],
            dad: vec![NIL; N + 1],
            match_position: 0,
            match_length: 0,
        };
        for i in N + 1..=N + 256 {
            t.rson[i] = NIL;
        }
        t
    }

    fn insert(&mut self, r: usize) {
        let mut cmp: i32 = 1;
        let mut p = N + 1 + self.text[r] as usize;
        self.rson[r] = NIL;
        self.lson[r] = NIL;
        self.match_length = 0;
        loop {
            if cmp >= 0 {
                if self.rson[p] != NIL {
                    p = self.rson[p];
                } else {
                    self.rson[p] = r;
                    self.dad[r] = p;
                    return;
                }
            } else if self.lson[p] != NIL {
                p = self.lson[p];
            } else {
                self.lson[p] = r;
                self.dad[r] = p;
                return;
            }
            let mut i = 1;
            while i < F {
                cmp = self.text[r + i] as i32 - self.text[p + i] as i32;
                if cmp != 0 {
                    break;
                }
                i += 1;
            }
            if i > THRESHOLD {
                if i > self.match_length {
                    self.match_position = (r.wrapping_sub(p)) & (N - 1);
                    self.match_length = i;
                    if i >= F {
                        break;
                    }
                } else if i == self.match_length {
                    let t = (r.wrapping_sub(p)) & (N - 1);
                    if t < self.match_position {
                        self.match_position = t;
                    }
                }
            }
        }
        self.dad[r] = self.dad[p];
        self.lson[r] = self.lson[p];
        self.rson[r] = self.rson[p];
        let (lp, rp) = (self.lson[p], self.rson[p]);
        self.dad[lp] = r;
        self.dad[rp] = r;
        let dp = self.dad[p];
        if self.rson[dp] == p {
            self.rson[dp] = r;
        } else {
            self.lson[dp] = r;
        }
        self.dad[p] = NIL;
    }

    fn delete(&mut self, p: usize) {
        if self.dad[p] == NIL {
            return;
        }
        let q;
        if self.rson[p] == NIL {
            q = self.lson[p];
        } else if self.lson[p] == NIL {
            q = self.rson[p];
        } else {
            let mut qq = self.lson[p];
            if self.rson[qq] != NIL {
                while self.rson[qq] != NIL {
                    qq = self.rson[qq];
                }
                let (dq, lq) = (self.dad[qq], self.lson[qq]);
                self.rson[dq] = lq;
                self.dad[lq] = dq;
                self.lson[qq] = self.lson[p];
                let lp = self.lson[p];
                self.dad[lp] = qq;
            }
            self.rson[qq] = self.rson[p];
            let rp = self.rson[p];
            self.dad[rp] = qq;
            q = qq;
        }
        self.dad[q] = self.dad[p];
        let dp = self.dad[p];
        if self.rson[dp] == p {
            self.rson[dp] = q;
        } else {
            self.lson[dp] = q;
        }
        self.dad[p] = NIL;
    }
}

/// Packs `input` the way LZARI.C does (its output is what the Action Replay software reads).
pub fn compress(input: &[u8]) -> Vec<u8> {
    let mut e = Encoder { out: (input.len() as u32).to_le_bytes().to_vec(), buffer: 0, mask: 128, low: 0, high: Q4, shifts: 0, m: Model::new() };
    if input.is_empty() {
        return e.out;
    }
    let mut t = Tree::new();
    let mut src = input.iter().copied();
    let mut s = 0;
    let mut r = N - F;
    for i in s..r {
        t.text[i] = b' ';
    }
    let mut len = 0;
    while len < F {
        let Some(c) = src.next() else { break };
        t.text[r + len] = c;
        len += 1;
    }
    for i in 1..=F {
        t.insert(r - i);
    }
    t.insert(r);
    loop {
        if t.match_length > len {
            t.match_length = len;
        }
        if t.match_length <= THRESHOLD {
            t.match_length = 1;
            e.char(t.text[r] as usize);
        } else {
            e.char(255 - THRESHOLD + t.match_length);
            e.position(t.match_position - 1);
        }
        let last = t.match_length;
        let mut i = 0;
        while i < last {
            let Some(c) = src.next() else { break };
            t.delete(s);
            t.text[s] = c;
            if s < F - 1 {
                t.text[s + N] = c;
            }
            s = (s + 1) & (N - 1);
            r = (r + 1) & (N - 1);
            t.insert(r);
            i += 1;
        }
        while i < last {
            i += 1;
            t.delete(s);
            s = (s + 1) & (N - 1);
            r = (r + 1) & (N - 1);
            len -= 1;
            if len > 0 {
                t.insert(r);
            }
        }
        if len == 0 {
            break;
        }
    }
    // EncodeEnd
    e.shifts += 1;
    let bit = e.low >= Q1;
    e.output(bit);
    for _ in 0..7 {
        e.put_bit(false);
    }
    e.out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let mut x: u32 = 7;
        let mut data: Vec<u8> = (0..20000u32).map(|i| if i % 7 < 3 { (i % 13) as u8 } else { b'A' + (i % 5) as u8 }).collect();
        for b in data.iter_mut().step_by(17) {
            x = x.wrapping_mul(1103515245).wrapping_add(12345);
            *b = (x >> 16) as u8;
        }
        let z = compress(&data);
        assert!(z.len() < data.len());
        assert_eq!(decompress(&z).unwrap(), data);
        assert_eq!(decompress(&compress(b"x")).unwrap(), b"x");
    }
}
