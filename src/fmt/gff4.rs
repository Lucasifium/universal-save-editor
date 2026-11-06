//! BioWare GFF V4.0 (Eclipse engine: Dragon Age), little-endian PC files. A header (magic,
//! version, platform, file type and version, template count, data offset), struct templates
//! (label, field count, field-list offset, size) and field definitions (numeric label, type,
//! flags, offset in the struct). Every struct is fixed size; strings, lists and referenced
//! structs live elsewhere in the data section, reached by data-relative offsets.
//!
//! Only reading is done here: callers find where a number lives and edit it in place with
//! `fields::Builder`. Layout facts from xoreos (GPL-3.0, facts only) and the MIT
//! dragon-age-save-editor by Aleksei Tovkachev.

use super::fields::Kind;

const LIST: u16 = 0x8000;
const STRUCT: u16 = 0x4000;
const REF: u16 = 0x2000;
const GENERIC: u16 = 0xFFFF;
const NONE: u32 = 0xFFFF_FFFF;

#[derive(Clone, Copy, Debug)]
pub struct Field {
    pub label: u32,
    pub ty: u16,
    pub flags: u16,
    pub off: usize,
}

#[derive(Clone, Debug)]
pub struct Template {
    pub label: [u8; 4],
    pub size: usize,
    pub fields: Vec<Field>,
}

/// A struct instance: its template and where it starts in the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct St {
    pub t: usize,
    pub at: usize,
}

pub struct Gff4<'a> {
    pub d: &'a [u8],
    /// Absolute offset of the data section (all stored offsets are relative to it).
    pub data: usize,
    pub templates: Vec<Template>,
}

fn u16_at(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

impl<'a> Gff4<'a> {
    /// A GFF V4.0 PC file of type `file_type` (e.g. `b"SAVE"`).
    pub fn open(d: &'a [u8], file_type: &[u8; 4]) -> Result<Self, String> {
        if d.len() < 0x1C || &d[0..8] != b"GFF V4.0" || &d[8..12] != b"PC  " || &d[12..16] != file_type {
            return Err("not a GFF V4.0 PC file".into());
        }
        let count = u32_at(d, 0x14).ok_or("short header")? as usize;
        let data = u32_at(d, 0x18).ok_or("short header")? as usize;
        if count == 0 || count > 10_000 || data > d.len() || 0x1C + 16 * count > d.len() {
            return Err("bad GFF header".into());
        }
        let mut templates = Vec::with_capacity(count);
        for i in 0..count {
            let t = 0x1C + 16 * i;
            let label: [u8; 4] = d[t..t + 4].try_into().unwrap();
            let (n, fo, size) = (u32_at(d, t + 4).unwrap() as usize, u32_at(d, t + 8).unwrap() as usize, u32_at(d, t + 12).unwrap() as usize);
            if n > 10_000 || fo.checked_add(12 * n).is_none_or(|e| e > d.len()) {
                return Err("bad struct template".into());
            }
            let fields = (0..n)
                .map(|k| {
                    let f = fo + 12 * k;
                    Field { label: u32_at(d, f).unwrap(), ty: u16_at(d, f + 4).unwrap(), flags: u16_at(d, f + 6).unwrap(), off: u32_at(d, f + 8).unwrap() as usize }
                })
                .collect::<Vec<_>>();
            if fields.iter().any(|f| f.off >= size) {
                return Err("field outside its struct".into());
            }
            templates.push(Template { label, size, fields });
        }
        if templates[0].size + data > d.len() {
            return Err("root struct outside the file".into());
        }
        Ok(Self { d, data, templates })
    }

    pub fn root(&self) -> St {
        St { t: 0, at: self.data }
    }

    pub fn label(&self, s: St) -> &[u8; 4] {
        &self.templates[s.t].label
    }

    /// A data-relative offset → absolute (None for "no value" or outside the file).
    fn rel(&self, off: u32) -> Option<usize> {
        if off == NONE {
            return None;
        }
        let at = self.data.checked_add(off as usize)?;
        (at < self.d.len()).then_some(at)
    }

    fn st(&self, t: usize, at: usize) -> Option<St> {
        let size = self.templates.get(t)?.size;
        (at.checked_add(size)? <= self.d.len()).then_some(St { t, at })
    }

    /// The field definition and the absolute position of its slot in `s`.
    pub fn field(&self, s: St, label: u32) -> Option<(Field, usize)> {
        let f = *self.templates[s.t].fields.iter().find(|f| f.label == label)?;
        Some((f, s.at + f.off))
    }

    /// A struct field (inline or by reference).
    pub fn child(&self, s: St, label: u32) -> Option<St> {
        let (f, at) = self.field(s, label)?;
        if f.flags & LIST != 0 || f.flags & STRUCT == 0 {
            return None;
        }
        let at = if f.flags & REF != 0 { self.rel(u32_at(self.d, at)?)? } else { at };
        self.st(f.ty as usize, at)
    }

    /// The structs of a list field: inline structs, references to structs, or generic
    /// entries (type, flags, offset) that are structs. Missing entries are skipped.
    pub fn list(&self, s: St, label: u32) -> Vec<St> {
        let Some((f, at)) = self.field(s, label) else { return vec![] };
        if f.flags & LIST == 0 {
            return vec![];
        }
        let Some(base) = u32_at(self.d, at).and_then(|o| self.rel(o)) else { return vec![] };
        let n = u32_at(self.d, base).unwrap_or(0) as usize;
        let first = base + 4;
        let mut out = vec![];
        for k in 0..n.min(1_000_000) {
            let st = if f.ty == GENERIC {
                let e = first + 8 * k;
                let (Some(t), Some(fl), Some(o)) = (u16_at(self.d, e), u16_at(self.d, e + 2), u32_at(self.d, e + 4)) else { break };
                if fl & STRUCT == 0 || fl & LIST != 0 {
                    continue;
                }
                self.rel(o).and_then(|a| self.st(t as usize, a))
            } else if f.flags & STRUCT == 0 {
                return vec![];
            } else if f.flags & REF != 0 {
                let Some(o) = u32_at(self.d, first + 4 * k) else { break };
                self.rel(o).and_then(|a| self.st(f.ty as usize, a))
            } else {
                let size = self.templates.get(f.ty as usize).map_or(0, |t| t.size);
                match self.st(f.ty as usize, first + size * k) {
                    Some(s) => Some(s),
                    None => break,
                }
            };
            if let Some(st) = st {
                out.push(st);
            }
        }
        out
    }

    /// Number of entries in a list field.
    pub fn list_len(&self, s: St, label: u32) -> usize {
        let Some((f, at)) = self.field(s, label) else { return 0 };
        if f.flags & LIST == 0 {
            return 0;
        }
        u32_at(self.d, at).and_then(|o| self.rel(o)).and_then(|b| u32_at(self.d, b)).unwrap_or(0) as usize
    }

    fn utf16(&self, at: usize) -> Option<String> {
        let n = u32_at(self.d, at)? as usize;
        let b = self.d.get(at + 4..at.checked_add(4 + 2 * n)?)?;
        let u: Vec<u16> = b.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&c| c != 0).collect();
        Some(String::from_utf16_lossy(&u))
    }

    /// A string field (not a reference, not a list).
    pub fn string(&self, s: St, label: u32) -> Option<String> {
        let (f, at) = self.field(s, label)?;
        if f.ty != 14 || f.flags != 0 {
            return None;
        }
        self.utf16(self.rel(u32_at(self.d, at)?)?)
    }

    /// A talk-table string: its string id and the text stored in the file, if any.
    pub fn tlk(&self, s: St, label: u32) -> Option<(u32, Option<String>)> {
        let (f, at) = self.field(s, label)?;
        if f.ty != 17 || f.flags != 0 {
            return None;
        }
        let id = u32_at(self.d, at)?;
        // Offset 0 is the root struct, so it never holds text: it means "none".
        let text = u32_at(self.d, at + 4).filter(|&o| o != 0).and_then(|o| self.rel(o)).and_then(|a| self.utf16(a));
        Some((id, text))
    }

    /// A plain number field: where it is and how to edit it.
    pub fn num(&self, s: St, label: u32) -> Option<(usize, Kind)> {
        let (f, at) = self.field(s, label)?;
        if f.flags != 0 {
            return None;
        }
        let k = match f.ty {
            0 => Kind::U8,
            2 => Kind::U16,
            3 => Kind::I16,
            4 => Kind::U32,
            5 => Kind::I32,
            8 => Kind::F32,
            _ => return None,
        };
        (at + k.size() <= self.d.len()).then_some((at, k))
    }

    /// A plain number field's value.
    pub fn value(&self, s: St, label: u32) -> Option<f64> {
        let (at, k) = self.num(s, label)?;
        super::fields::read(self.d, at, k, false)?.as_f64()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Builds a GFF V4.0 PC file: `templates` = (label, size, fields (label, type, flags, offset)),
    /// `data` = the data section (the root struct first).
    pub fn build(file_type: &[u8; 4], version: &[u8; 4], templates: &[(&[u8; 4], u32, Vec<(u32, u16, u16, u32)>)], data: &[u8]) -> Vec<u8> {
        let mut d = vec![];
        d.extend(b"GFF V4.0PC  ");
        d.extend(file_type);
        d.extend(version);
        d.extend((templates.len() as u32).to_le_bytes());
        let fields_at = 0x1C + 16 * templates.len();
        let nfields: usize = templates.iter().map(|t| t.2.len()).sum();
        let data_at = fields_at + 12 * nfields;
        d.extend((data_at as u32).to_le_bytes());
        let mut fo = fields_at;
        for (label, size, fields) in templates {
            d.extend(*label);
            d.extend((fields.len() as u32).to_le_bytes());
            d.extend((fo as u32).to_le_bytes());
            d.extend(size.to_le_bytes());
            fo += 12 * fields.len();
        }
        for (_, _, fields) in templates {
            for (l, t, fl, o) in fields {
                d.extend(l.to_le_bytes());
                d.extend(t.to_le_bytes());
                d.extend(fl.to_le_bytes());
                d.extend(o.to_le_bytes());
            }
        }
        d.extend(data);
        d
    }

    #[test]
    fn reads_structs_lists_and_strings() {
        // Root (8 B): u32 at 0, inline-struct list at 4. Item (8 B): i32 at 0, string at 4.
        let mut data = vec![0u8; 8];
        data[0..4].copy_from_slice(&77u32.to_le_bytes());
        data[4..8].copy_from_slice(&8u32.to_le_bytes());
        data.extend(2u32.to_le_bytes());
        for (n, s_off) in [(5i32, 28u32), (6, NONE)] {
            data.extend(n.to_le_bytes());
            data.extend(s_off.to_le_bytes());
        }
        data.extend(3u32.to_le_bytes());
        for c in "ab\0".encode_utf16() {
            data.extend(c.to_le_bytes());
        }
        let f = build(b"TEST", b"V1.0", &[(b"ROOT", 8, vec![(1, 4, 0, 0), (2, 1, 0xC000, 4)]), (b"ITEM", 8, vec![(3, 5, 0, 0), (4, 14, 0, 4)])], &data);
        let g = Gff4::open(&f, b"TEST").unwrap();
        assert!(Gff4::open(&f, b"SAVE").is_err());
        let r = g.root();
        assert_eq!(g.value(r, 1), Some(77.0));
        let items = g.list(r, 2);
        assert_eq!(items.len(), 2);
        assert_eq!(g.list_len(r, 2), 2);
        assert_eq!(g.label(items[0]), b"ITEM");
        assert_eq!(g.value(items[1], 3), Some(6.0));
        assert_eq!(g.string(items[0], 4).as_deref(), Some("ab"));
        assert_eq!(g.string(items[1], 4), None);
        assert_eq!(g.num(items[0], 3).map(|x| x.1), Some(Kind::I32));
    }
}
