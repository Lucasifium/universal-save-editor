//! BioWare's Aurora / Odyssey files: GFF V3.2 (a tree of labelled fields) and the ERF
//! archive around them (`.sav`, `.mod`, `.erf`). Star Wars: KOTOR 1 and 2 and Neverwinter
//! Nights keep their saves this way; `kotor.rs` and `nwn.rs` pick the values out.
//!
//! Written from the public BioWare Aurora format descriptions (GFF and ERF), checked on real
//! saves. Everything here only reads; numbers are edited in place by `fields::Builder`
//! (a number's place never moves, so a GFF stays valid without being rebuilt).
//!
//! GFF: a 56-byte header (type, "V3.2", then offset/count pairs of the struct, field,
//! label, field data, field index and list index blocks). A struct (12 bytes) is a type,
//! one field index or an offset into the field indices, and a field count. A field
//! (12 bytes) is its type, label index and a 4-byte slot: small numbers live in the slot,
//! bigger values in the field data block at the slot's offset, a struct is its index and a
//! list is an offset into the list indices (a count and struct indices).

use super::fields::{Builder, Kind};
use serde_json::Value;

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// One GFF inside a file (`base` is where it starts; all offsets below are absolute).
#[derive(Clone, Copy)]
pub struct Gff<'a> {
    pub d: &'a [u8],
    pub file_type: [u8; 4],
    structs: (usize, usize),
    fields: (usize, usize),
    labels: (usize, usize),
    data: (usize, usize),
    findex: (usize, usize),
    lindex: (usize, usize),
}

/// A field: its type, label and where its 4-byte slot is.
#[derive(Clone, Copy, Debug)]
pub struct Field {
    pub ty: u32,
    pub label: u32,
    pub raw: u32,
    pub slot: usize,
}

impl<'a> Gff<'a> {
    /// The GFF at `base` in `d` (bounded by `end`), if its header holds together.
    pub fn open(d: &'a [u8], base: usize, end: usize) -> Option<Self> {
        let end = end.min(d.len());
        let h = d.get(base..base + 56)?;
        if &h[4..8] != b"V3.2" {
            return None;
        }
        let pair = |i: usize| -> Option<(usize, usize)> {
            let off = u32_at(h, 8 + 8 * i)? as usize;
            let n = u32_at(h, 12 + 8 * i)? as usize;
            Some((base + off, n))
        };
        let structs = pair(0)?;
        let fields = pair(1)?;
        let labels = pair(2)?;
        let data = pair(3)?;
        let findex = pair(4)?;
        let lindex = pair(5)?;
        let fits = |(at, len): (usize, usize)| at.checked_add(len).is_some_and(|e| e <= end) && at >= base + 56;
        if structs.1 == 0
            || !fits((structs.0, structs.1.checked_mul(12)?))
            || !fits((fields.0, fields.1.checked_mul(12)?))
            || !fits((labels.0, labels.1.checked_mul(16)?))
            || !fits(data)
            || !fits(findex)
            || !fits(lindex)
        {
            return None;
        }
        Some(Self { d, file_type: h[0..4].try_into().ok()?, structs, fields, labels, data, findex, lindex })
    }

    fn label_bytes(&self, i: u32) -> Option<&'a [u8]> {
        if i as usize >= self.labels.1 {
            return None;
        }
        let at = self.labels.0 + 16 * i as usize;
        let l = self.d.get(at..at + 16)?;
        Some(&l[..l.iter().position(|&c| c == 0).unwrap_or(16)])
    }

    fn field(&self, i: u32) -> Option<Field> {
        if i as usize >= self.fields.1 {
            return None;
        }
        let at = self.fields.0 + 12 * i as usize;
        Some(Field { ty: u32_at(self.d, at)?, label: u32_at(self.d, at + 4)?, raw: u32_at(self.d, at + 8)?, slot: at + 8 })
    }

    /// The fields of struct `s` (0 is the root).
    pub fn fields_of(&self, s: u32) -> Vec<Field> {
        let mut out = vec![];
        if s as usize >= self.structs.1 {
            return out;
        }
        let at = self.structs.0 + 12 * s as usize;
        let (Some(v), Some(n)) = (u32_at(self.d, at + 4), u32_at(self.d, at + 8)) else { return out };
        match n {
            0 => {}
            1 => out.extend(self.field(v)),
            n => {
                let first = v as usize;
                if (n as usize).checked_mul(4).is_none_or(|len| first + len > self.findex.1) {
                    return out;
                }
                for k in 0..n as usize {
                    if let Some(f) = u32_at(self.d, self.findex.0 + first + 4 * k).and_then(|i| self.field(i)) {
                        out.push(f);
                    }
                }
            }
        }
        out
    }

    /// The field labelled `label` in struct `s`.
    pub fn get(&self, s: u32, label: &str) -> Option<Field> {
        let want = label.as_bytes();
        self.fields_of(s).into_iter().find(|f| self.label_bytes(f.label) == Some(want))
    }

    fn data_at(&self, f: &Field, len: usize) -> Option<usize> {
        let at = self.data.0 + f.raw as usize;
        (f.raw as usize + len <= self.data.1).then_some(at)
    }

    /// Where a number is stored and how wide it is (`None` for strings, lists, structs).
    pub fn number(&self, f: &Field) -> Option<(usize, Kind)> {
        Some(match f.ty {
            0 => (f.slot, Kind::U8),
            2 => (f.slot, Kind::U16),
            3 => (f.slot, Kind::I16),
            4 => (f.slot, Kind::U32),
            5 => (f.slot, Kind::I32),
            6 => (self.data_at(f, 8)?, Kind::U64),
            8 => (f.slot, Kind::F32),
            _ => return None,
        })
    }

    /// Where a VOID field's bytes start in the file and how many there are.
    pub fn void_of(&self, f: &Field) -> Option<(usize, usize)> {
        if f.ty != 13 {
            return None;
        }
        let at = self.data_at(f, 4)?;
        let n = u32_at(self.d, at)? as usize;
        self.data_at(f, 4 + n)?;
        Some((at + 4, n))
    }

    /// A whole number's value (BYTE … INT, DWORD64, and CHAR as signed).
    pub fn int_of(&self, f: &Field) -> Option<i64> {
        if f.ty == 1 {
            return Some(i64::from(*self.d.get(f.slot)? as i8));
        }
        let (at, kind) = self.number(f)?;
        if kind == Kind::F32 {
            return None;
        }
        super::fields::read(self.d, at, kind, false)?.as_i64()
    }

    pub fn int(&self, s: u32, label: &str) -> Option<i64> {
        self.int_of(&self.get(s, label)?)
    }

    /// A string field: CExoString, ResRef, or a CExoLocString's first embedded text.
    pub fn text_of(&self, f: &Field) -> Option<String> {
        let latin = |b: &[u8]| b.iter().map(|&c| super::bethesda_cp1252(c)).collect::<String>();
        match f.ty {
            10 => {
                let at = self.data_at(f, 4)?;
                let n = u32_at(self.d, at)? as usize;
                self.data_at(f, 4 + n)?;
                Some(latin(self.d.get(at + 4..at + 4 + n)?))
            }
            11 => {
                let at = self.data_at(f, 1)?;
                let n = *self.d.get(at)? as usize;
                self.data_at(f, 1 + n)?;
                Some(latin(self.d.get(at + 1..at + 1 + n)?))
            }
            12 => {
                let at = self.data_at(f, 12)?;
                let count = u32_at(self.d, at + 8)?;
                if count == 0 {
                    return None;
                }
                let n = u32_at(self.d, at + 16)? as usize;
                self.data_at(f, 20 + n)?;
                Some(latin(self.d.get(at + 20..at + 20 + n)?))
            }
            _ => None,
        }
    }

    pub fn text(&self, s: u32, label: &str) -> Option<String> {
        self.text_of(&self.get(s, label)?)
    }

    /// The struct indices of a list field.
    pub fn list(&self, s: u32, label: &str) -> Vec<u32> {
        let Some(f) = self.get(s, label).filter(|f| f.ty == 15) else { return vec![] };
        let at = f.raw as usize;
        let Some(n) = u32_at(self.d, self.lindex.0 + at) else { return vec![] };
        if at + 4 + 4 * n as usize > self.lindex.1 {
            return vec![];
        }
        (0..n as usize).filter_map(|k| u32_at(self.d, self.lindex.0 + at + 4 + 4 * k)).filter(|&i| (i as usize) < self.structs.1).collect()
    }

    /// A number as an editable value at `path`, limited to `lo..=hi` (on top of its type).
    pub fn edit(&self, b: &mut Builder, path: &[&str], s: u32, label: &str, lo: i64, hi: i64) -> Option<Value> {
        let f = self.get(s, label)?;
        let (at, kind) = self.number(&f)?;
        if kind == Kind::F32 {
            return b.field(path, at, kind);
        }
        b.ranged(path, at, kind, lo, hi)
    }

    /// Where the same field of another copy is, if it has the same type (for `Builder::mirror`).
    pub fn same_place(&self, s: u32, label: &str, like: &Field) -> Option<usize> {
        let f = self.get(s, label).filter(|f| f.ty == like.ty)?;
        self.number(&f).map(|x| x.0)
    }
}

/// One struct of a GFF (a creature, an item…).
#[derive(Clone, Copy)]
pub struct At<'a> {
    pub g: Gff<'a>,
    pub s: u32,
}

impl<'a> At<'a> {
    pub fn root(g: Gff<'a>) -> Self {
        Self { g, s: 0 }
    }

    pub fn int(&self, label: &str) -> Option<i64> {
        self.g.int(self.s, label)
    }

    pub fn text(&self, label: &str) -> Option<String> {
        self.g.text(self.s, label)
    }

    pub fn has(&self, label: &str) -> bool {
        self.g.get(self.s, label).is_some()
    }

    pub fn list(&self, label: &str) -> Vec<At<'a>> {
        self.g.list(self.s, label).into_iter().map(|s| At { g: self.g, s }).collect()
    }

    /// `label` of this struct as an editable number at `path`; the same field of each of
    /// `copies` (other places the game keeps the same value) is written along with it.
    pub fn edit(&self, b: &mut Builder, path: &[&str], label: &str, lo: i64, hi: i64, copies: &[At]) -> Option<Value> {
        let v = self.g.edit(b, path, self.s, label, lo, hi)?;
        let f = self.g.get(self.s, label)?;
        for c in copies {
            if let Some(at) = c.g.same_place(c.s, label, &f) {
                b.mirror(path, at);
            }
        }
        Some(v)
    }
}

/// One file inside an ERF archive (offsets absolute in the outer file).
#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub ty: u16,
    pub at: usize,
    pub size: usize,
}

/// Resource types this code looks for.
pub const T_IFO: u16 = 2014;
pub const T_UTC: u16 = 2027;
pub const T_SAV: u16 = 2057;

/// True for an ERF-family header ("MOD V1.0", "SAV V1.0", "ERF V1.0").
pub fn is_erf(d: &[u8], base: usize) -> bool {
    d.get(base..base + 8).is_some_and(|h| matches!(&h[..4], b"MOD " | b"SAV " | b"ERF ") && &h[4..] == b"V1.0")
}

/// The entries of the ERF at `base` (its own offsets are relative to `base`).
pub fn erf_entries(d: &[u8], base: usize, end: usize) -> Option<Vec<Entry>> {
    if !is_erf(d, base) {
        return None;
    }
    let end = end.min(d.len());
    let n = u32_at(d, base + 0x10)? as usize;
    let keys = base + u32_at(d, base + 0x18)? as usize;
    let res = base + u32_at(d, base + 0x1C)? as usize;
    if n > 100_000 || keys + 24 * n > end || res + 8 * n > end {
        return None;
    }
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let k = keys + 24 * i;
        let raw = &d[k..k + 16];
        let name: String = raw[..raw.iter().position(|&c| c == 0).unwrap_or(16)].iter().map(|&c| c as char).collect();
        let ty = u16::from_le_bytes([d[k + 20], d[k + 21]]);
        let at = base + u32_at(d, res + 8 * i)? as usize;
        let size = u32_at(d, res + 8 * i + 4)? as usize;
        if at.checked_add(size).is_none_or(|e| e > end) {
            return None;
        }
        out.push(Entry { name, ty, at, size });
    }
    Some(out)
}

/// The GFF stored as `e`.
pub fn entry_gff<'a>(d: &'a [u8], e: &Entry) -> Option<Gff<'a>> {
    Gff::open(d, e.at, e.at + e.size)
}

#[cfg(test)]
pub mod build {
    //! Writes small GFF and ERF files for tests (synthetic saves).

    pub enum V {
        Byte(u8),
        Word(u16),
        Short(i16),
        Dword(u32),
        Int(i32),
        Str(&'static str),
        Void(Vec<u8>),
        Loc(i32, &'static str),
        Struct(u32, Vec<(&'static str, V)>),
        List(Vec<(u32, Vec<(&'static str, V)>)>),
    }

    #[derive(Default)]
    struct W {
        structs: Vec<[u32; 3]>,
        fields: Vec<[u32; 3]>,
        labels: Vec<String>,
        data: Vec<u8>,
        findex: Vec<u8>,
        lindex: Vec<u8>,
    }

    impl W {
        fn label(&mut self, l: &str) -> u32 {
            match self.labels.iter().position(|x| x == l) {
                Some(i) => i as u32,
                None => {
                    self.labels.push(l.to_string());
                    self.labels.len() as u32 - 1
                }
            }
        }

        fn add_struct(&mut self, ty: u32, fields: &[(&'static str, V)]) -> u32 {
            let si = self.structs.len();
            self.structs.push([ty, 0, fields.len() as u32]);
            let mut idx = vec![];
            for (l, v) in fields {
                let li = self.label(l);
                let (t, raw) = match v {
                    V::Byte(x) => (0, u32::from(*x)),
                    V::Word(x) => (2, u32::from(*x)),
                    V::Short(x) => (3, *x as u16 as u32),
                    V::Dword(x) => (4, *x),
                    V::Int(x) => (5, *x as u32),
                    V::Str(s) => {
                        let at = self.data.len() as u32;
                        self.data.extend((s.len() as u32).to_le_bytes());
                        self.data.extend(s.as_bytes());
                        (10, at)
                    }
                    V::Void(b) => {
                        let at = self.data.len() as u32;
                        self.data.extend((b.len() as u32).to_le_bytes());
                        self.data.extend(b);
                        (13, at)
                    }
                    V::Loc(strref, s) => {
                        let at = self.data.len() as u32;
                        let n = if s.is_empty() { 0 } else { 1 };
                        let body = 8 + if n == 1 { 8 + s.len() } else { 0 };
                        self.data.extend((body as u32).to_le_bytes());
                        self.data.extend(strref.to_le_bytes());
                        self.data.extend((n as u32).to_le_bytes());
                        if n == 1 {
                            self.data.extend(0u32.to_le_bytes());
                            self.data.extend((s.len() as u32).to_le_bytes());
                            self.data.extend(s.as_bytes());
                        }
                        (12, at)
                    }
                    V::Struct(ty, f) => (14, self.add_struct(*ty, f)),
                    V::List(items) => {
                        let kids: Vec<u32> = items.iter().map(|(ty, f)| self.add_struct(*ty, f)).collect();
                        let at = self.lindex.len() as u32;
                        self.lindex.extend((kids.len() as u32).to_le_bytes());
                        for k in kids {
                            self.lindex.extend(k.to_le_bytes());
                        }
                        (15, at)
                    }
                };
                self.fields.push([t, li, raw]);
                idx.push(self.fields.len() as u32 - 1);
            }
            match idx.len() {
                0 => {}
                1 => self.structs[si][1] = idx[0],
                _ => {
                    self.structs[si][1] = self.findex.len() as u32;
                    for i in idx {
                        self.findex.extend(i.to_le_bytes());
                    }
                }
            }
            si as u32
        }
    }

    /// A GFF file of type `ty` (four characters) with these root fields.
    pub fn gff(ty: &str, root: Vec<(&'static str, V)>) -> Vec<u8> {
        let mut w = W::default();
        w.add_struct(u32::MAX, &root);
        let mut body = vec![];
        let mut offs = vec![];
        let blocks: Vec<(Vec<u8>, usize)> = vec![
            (w.structs.iter().flat_map(|s| s.iter().flat_map(|x| x.to_le_bytes())).collect(), w.structs.len()),
            (w.fields.iter().flat_map(|s| s.iter().flat_map(|x| x.to_le_bytes())).collect(), w.fields.len()),
            (w.labels.iter().flat_map(|l| { let mut b = l.as_bytes().to_vec(); b.resize(16, 0); b }).collect(), w.labels.len()),
            (w.data.clone(), w.data.len()),
            (w.findex.clone(), w.findex.len()),
            (w.lindex.clone(), w.lindex.len()),
        ];
        for (bytes, count) in blocks {
            offs.push((56 + body.len()) as u32);
            offs.push(count as u32);
            body.extend(bytes);
        }
        let mut out = ty.as_bytes().to_vec();
        out.extend(b"V3.2");
        for v in offs {
            out.extend(v.to_le_bytes());
        }
        out.extend(body);
        out
    }

    /// An ERF ("MOD V1.0") holding these (name, type, bytes).
    pub fn erf(files: &[(&str, u16, Vec<u8>)]) -> Vec<u8> {
        let n = files.len();
        let keys = 160;
        let res = keys + 24 * n;
        let mut at = res + 8 * n;
        let mut out = b"MOD V1.0".to_vec();
        for v in [0u32, 0, n as u32, keys as u32, keys as u32, res as u32] {
            out.extend(v.to_le_bytes());
        }
        out.resize(160, 0);
        for (i, (name, ty, _)) in files.iter().enumerate() {
            let mut k = name.as_bytes().to_vec();
            k.resize(16, 0);
            out.extend(k);
            out.extend((i as u32).to_le_bytes());
            out.extend(ty.to_le_bytes());
            out.extend([0, 0]);
        }
        for (_, _, d) in files {
            out.extend((at as u32).to_le_bytes());
            out.extend((d.len() as u32).to_le_bytes());
            at += d.len();
        }
        for (_, _, d) in files {
            out.extend(d);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::build::{erf, gff, V};
    use super::*;

    #[test]
    fn reads_fields_lists_and_nested_archives() {
        let g = gff(
            "UTC ",
            vec![
                ("Str", V::Byte(14)),
                ("HitPoints", V::Short(-3)),
                ("Experience", V::Dword(1234)),
                ("Tag", V::Str("hero")),
                ("FirstName", V::Loc(-1, "Revan")),
                ("SkillList", V::List(vec![(0, vec![("Rank", V::Byte(4))]), (0, vec![("Rank", V::Byte(7))])])),
                ("Info", V::Struct(9, vec![("Inner", V::Int(-5))])),
            ],
        );
        let outer = erf(&[("first", T_UTC, vec![1, 2, 3]), ("module", T_SAV, erf(&[("PC", T_UTC, g.clone())]))]);
        let top = erf_entries(&outer, 0, outer.len()).unwrap();
        assert_eq!(top[1].name, "module");
        let inner = erf_entries(&outer, top[1].at, top[1].at + top[1].size).unwrap();
        let c = entry_gff(&outer, &inner[0]).unwrap();
        assert_eq!(&c.file_type, b"UTC ");
        assert_eq!(c.int(0, "Str"), Some(14));
        assert_eq!(c.int(0, "HitPoints"), Some(-3));
        assert_eq!(c.int(0, "Experience"), Some(1234));
        assert_eq!(c.text(0, "Tag").as_deref(), Some("hero"));
        assert_eq!(c.text(0, "FirstName").as_deref(), Some("Revan"));
        let ranks: Vec<i64> = c.list(0, "SkillList").iter().filter_map(|&s| c.int(s, "Rank")).collect();
        assert_eq!(ranks, vec![4, 7]);
        let info = c.get(0, "Info").unwrap();
        assert_eq!((info.ty, c.int(info.raw, "Inner")), (14, Some(-5)));
        // Absolute places: the value really is where `number` says.
        let (at, kind) = c.number(&c.get(0, "Experience").unwrap()).unwrap();
        assert_eq!((kind, &outer[at..at + 4]), (Kind::U32, &1234u32.to_le_bytes()[..]));
        // Broken headers are refused, not read out of bounds.
        assert!(Gff::open(&g[..40], 0, 40).is_none());
        let mut bad = g.clone();
        bad[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(Gff::open(&bad, 0, bad.len()).is_none());
    }
}
