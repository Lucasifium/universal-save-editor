//! Larian's binary resource format LSF ("LSOF"), used for the game state inside `.lsv`
//! saves: a header, then four packed sections — names (a hash table of strings), nodes,
//! attributes and values (plus keys in newer files).
//!
//! The editor works on an *unpacked* LSF: the same header with every section stored
//! (its packed size set to 0, which LSF readers take as "not compressed"), so values can
//! be changed in place at fixed offsets. `pack` turns it back into the game's form,
//! reusing the original packed bytes for every section that didn't change.
//!
//! Layout ported from LSLib (Norbyte, MIT): `LSFCommon.cs`, `LSFReader.cs`.

use super::lspk;

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn i32_at(d: &[u8], at: usize) -> Option<i32> {
    u32_at(d, at).map(|v| v as i32)
}

/// Header facts: where the size pairs are and how long the header is.
struct Head {
    version: u32,
    /// Offsets of the (unpacked size, packed size) pairs, in the order the sections follow.
    pairs: Vec<usize>,
    flags: u8,
    metadata_format: u32,
    len: usize,
}

fn head(d: &[u8]) -> Option<Head> {
    if !d.starts_with(b"LSOF") {
        return None;
    }
    let version = u32_at(d, 4)?;
    if !(1..=7).contains(&version) {
        return None;
    }
    let m = if version >= 5 { 16 } else { 12 };
    // strings, [keys], nodes, attributes, values
    let n = if version >= 6 { 5 } else { 4 };
    let flags = *d.get(m + 8 * n)?;
    let metadata_format = u32_at(d, m + 8 * n + 4)?;
    let at = |i: usize| m + 8 * i;
    // Sections are stored strings, nodes, attributes, values, keys.
    let pairs = if version >= 6 {
        vec![at(0), at(2), at(3), at(4), at(1)]
    } else {
        vec![at(0), at(1), at(2), at(3)]
    };
    Some(Head {
        version,
        pairs,
        flags,
        metadata_format,
        len: m + 8 * n + 8,
    })
}

/// The sections as (unpacked size, packed size, packed bytes), in file order; keys only
/// when the file has them.
fn sections<'a>(d: &'a [u8], h: &Head) -> Option<Vec<(usize, usize, &'a [u8])>> {
    let mut at = h.len;
    let mut out = vec![];
    for (i, &p) in h.pairs.iter().enumerate() {
        if i == 4 && h.metadata_format != 1 {
            out.push((0, 0, &d[0..0]));
            continue;
        }
        let (us, ps) = (u32_at(d, p)? as usize, u32_at(d, p + 4)? as usize);
        // Packed size 0 (or no compression) means the section is stored as it is.
        let n = if ps == 0 || h.flags & 0x0F == 0 {
            us
        } else {
            ps
        };
        out.push((us, ps, d.get(at..at + n)?));
        at += n;
    }
    Some(out)
}

fn unpack_section(h: &Head, i: usize, (us, ps, b): (usize, usize, &[u8])) -> Option<Vec<u8>> {
    if ps == 0 || h.flags & 0x0F == 0 {
        return (b.len() == us).then(|| b.to_vec());
    }
    // The names section is one LZ4 block; the others are LZ4 frames from version 2 on.
    lspk::decompress(b, h.flags, us, i != 0 && h.version >= 2)
}

/// The unpacked form of an LSF file (every section stored as it is).
pub fn unpack(d: &[u8]) -> Option<Vec<u8>> {
    let h = head(d)?;
    let secs = sections(d, &h)?;
    let mut out = d[..h.len].to_vec();
    for (i, s) in secs.into_iter().enumerate() {
        let plain = unpack_section(&h, i, s)?;
        if i < 4 || h.metadata_format == 1 {
            out[h.pairs[i] + 4..h.pairs[i] + 8].fill(0);
        }
        out.extend(plain);
    }
    Some(out)
}

/// Packs an unpacked LSF again. Sections equal to the ones in `original` keep its packed
/// bytes; changed ones are packed with the file's own method.
pub fn pack(plain: &[u8], original: &[u8]) -> Result<Vec<u8>, String> {
    let bad = || "the game data no longer reads as an LSF resource".to_string();
    let h = head(plain).ok_or_else(bad)?;
    let oh = head(original).ok_or_else(bad)?;
    let ps = sections(plain, &h).ok_or_else(bad)?;
    let os = sections(original, &oh).ok_or_else(bad)?;
    let mut out = plain[..h.len].to_vec();
    for (i, s) in ps.into_iter().enumerate() {
        if i == 4 && h.metadata_format != 1 {
            continue;
        }
        let new = s.2;
        let old = os
            .get(i)
            .copied()
            .and_then(|o| unpack_section(&oh, i, o).map(|p| (o, p)));
        let (ps_new, bytes) = match old {
            Some((o, p)) if p == new => (o.1, o.2.to_vec()),
            _ if h.flags & 0x0F == 0 || new.is_empty() => (0, new.to_vec()),
            _ => {
                let c = lspk::compress(new, h.flags, i != 0 && h.version >= 2)?;
                (c.len(), c)
            }
        };
        out[h.pairs[i]..h.pairs[i] + 4].copy_from_slice(&(new.len() as u32).to_le_bytes());
        out[h.pairs[i] + 4..h.pairs[i] + 8].copy_from_slice(&(ps_new as u32).to_le_bytes());
        out.extend(bytes);
    }
    Ok(out)
}

/// A typed value.
#[derive(Clone, Copy, Debug)]
pub struct Attr {
    pub name: u32,
    pub ty: u32,
    pub len: usize,
    /// Absolute offset of the value in the unpacked file.
    pub at: usize,
    next: i32,
}

/// A parsed, unpacked LSF: node tree and attributes, with value positions.
pub struct Lsf<'a> {
    pub d: &'a [u8],
    names: Vec<Vec<String>>,
    /// (name, parent) per node.
    nodes: Vec<(u32, i32)>,
    first_attr: Vec<i32>,
    first_child: Vec<i32>,
    next_sibling: Vec<i32>,
    attrs: Vec<Attr>,
}

impl<'a> Lsf<'a> {
    /// Reads an unpacked LSF (see `unpack`).
    pub fn read(d: &'a [u8]) -> Option<Self> {
        let h = head(d)?;
        let secs = sections(d, &h)?;
        if secs.iter().take(4).any(|s| s.1 != 0 && h.flags & 0x0F != 0) {
            return None;
        }
        let base = |i: usize| secs[i].2.as_ptr() as usize - d.as_ptr() as usize;
        let (sn, nb, ab, vb) = (secs[0].2, secs[1].2, secs[2].2, secs[3].2);
        let values_at = base(3);
        // Names: bucket count, then per bucket a count and (length, UTF-8) strings.
        let mut names = vec![];
        let mut p = 4;
        for _ in 0..u32_at(sn, 0)? {
            let n = u16::from_le_bytes(sn.get(p..p + 2)?.try_into().ok()?) as usize;
            p += 2;
            let mut bucket = Vec::with_capacity(n);
            for _ in 0..n {
                let l = u16::from_le_bytes(sn.get(p..p + 2)?.try_into().ok()?) as usize;
                bucket.push(String::from_utf8_lossy(sn.get(p + 2..p + 2 + l)?).into_owned());
                p += 2 + l;
            }
            names.push(bucket);
        }
        let v3 = h.version >= 3 && h.metadata_format == 1;
        let nsz = if v3 { 16 } else { 12 };
        let count = nb.len() / nsz;
        let mut nodes = Vec::with_capacity(count);
        let mut first_attr = vec![-1; count];
        for i in 0..count {
            let e = i * nsz;
            let (name, parent, first) = if v3 {
                (u32_at(nb, e)?, i32_at(nb, e + 4)?, i32_at(nb, e + 12)?)
            } else {
                (u32_at(nb, e)?, i32_at(nb, e + 8)?, i32_at(nb, e + 4)?)
            };
            if parent >= i as i32 {
                return None;
            }
            nodes.push((name, parent));
            first_attr[i] = first;
        }
        let asz = if v3 { 16 } else { 12 };
        let acount = ab.len() / asz;
        let mut attrs: Vec<Attr> = Vec::with_capacity(acount);
        let mut offset = 0usize;
        let mut last: Vec<i32> = vec![-1; count];
        for i in 0..acount {
            let e = i * asz;
            let (name, tl) = (u32_at(ab, e)?, u32_at(ab, e + 4)?);
            let (ty, len) = (tl & 0x3F, (tl >> 6) as usize);
            if v3 {
                let at = u32_at(ab, e + 12)? as usize;
                attrs.push(Attr {
                    name,
                    ty,
                    len,
                    at: values_at + at,
                    next: i32_at(ab, e + 8)?,
                });
            } else {
                // Values follow each other; a node's attributes are those naming it, in order.
                let node = i32_at(ab, e + 8)?;
                attrs.push(Attr {
                    name,
                    ty,
                    len,
                    at: values_at + offset,
                    next: -1,
                });
                offset += len;
                let n = usize::try_from(node).ok().filter(|&n| n < count)?;
                match last[n] {
                    -1 => first_attr[n] = i as i32,
                    l => attrs[l as usize].next = i as i32,
                }
                last[n] = i as i32;
            }
        }
        if attrs.iter().any(|a| a.at + a.len > values_at + vb.len()) {
            return None;
        }
        let mut first_child = vec![-1; count];
        let mut next_sibling = vec![-1; count];
        for i in (0..count).rev() {
            let p = nodes[i].1;
            if p >= 0 {
                next_sibling[i] = first_child[p as usize];
                first_child[p as usize] = i as i32;
            }
        }
        let lsf = Lsf {
            d,
            names,
            nodes,
            first_attr,
            first_child,
            next_sibling,
            attrs,
        };
        // Every name reference must resolve.
        if lsf
            .nodes
            .iter()
            .map(|n| n.0)
            .chain(lsf.attrs.iter().map(|a| a.name))
            .any(|r| lsf.name_of(r).is_none())
        {
            return None;
        }
        Some(lsf)
    }

    fn name_of(&self, r: u32) -> Option<&str> {
        self.names
            .get((r >> 16) as usize)?
            .get((r & 0xFFFF) as usize)
            .map(String::as_str)
    }

    pub fn name(&self, r: u32) -> &str {
        self.name_of(r).unwrap_or("")
    }

    pub fn node_name(&self, n: usize) -> &str {
        self.name(self.nodes[n].0)
    }

    /// Top-level nodes (regions).
    pub fn roots(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.nodes.len()).filter(|&i| self.nodes[i].1 < 0)
    }

    pub fn children(&self, n: usize) -> impl Iterator<Item = usize> + '_ {
        let mut c = self.first_child[n];
        std::iter::from_fn(move || {
            let i = usize::try_from(c).ok()?;
            c = self.next_sibling[i];
            Some(i)
        })
    }

    pub fn children_named<'s>(
        &'s self,
        n: usize,
        name: &'s str,
    ) -> impl Iterator<Item = usize> + 's {
        self.children(n).filter(move |&c| self.node_name(c) == name)
    }

    pub fn child(&self, n: usize, name: &str) -> Option<usize> {
        self.children_named(n, name).next()
    }

    /// Every node at this path of names from a region.
    pub fn at_path(&self, path: &[&str]) -> Vec<usize> {
        let mut cur: Vec<usize> = self
            .roots()
            .filter(|&r| self.node_name(r) == path[0])
            .collect();
        for name in &path[1..] {
            cur = cur
                .iter()
                .flat_map(|&n| self.children_named(n, name).collect::<Vec<_>>())
                .collect();
        }
        cur
    }

    pub fn attrs(&self, n: usize) -> impl Iterator<Item = &Attr> + '_ {
        let mut a = self.first_attr[n];
        std::iter::from_fn(move || {
            let x = self.attrs.get(usize::try_from(a).ok()?)?;
            a = x.next;
            Some(x)
        })
    }

    pub fn attr(&self, n: usize, name: &str) -> Option<Attr> {
        self.attrs(n).find(|a| self.name(a.name) == name).copied()
    }

    /// An integer or bool value.
    pub fn int(&self, a: &Attr) -> Option<i64> {
        let b = self.d.get(a.at..a.at + a.len)?;
        let le = |n: usize| -> Option<[u8; 8]> {
            let mut x = [0u8; 8];
            x[..n].copy_from_slice(b.get(..n)?);
            Some(x)
        };
        Some(match a.ty {
            1 | 19 => i64::from(b[0]),
            27 => i64::from(b[0] as i8),
            2 => i64::from(i16::from_le_bytes(b.get(..2)?.try_into().ok()?)),
            3 => i64::from(u16::from_le_bytes(b.get(..2)?.try_into().ok()?)),
            4 => i64::from(i32::from_le_bytes(b.get(..4)?.try_into().ok()?)),
            5 => i64::from(u32::from_le_bytes(b.get(..4)?.try_into().ok()?)),
            24 | 26 | 32 => i64::from_le_bytes(le(8)?),
            _ => return None,
        })
    }

    /// A string value (String, Path, FixedString, LSString, WString, LSWString).
    pub fn string(&self, a: &Attr) -> Option<String> {
        if !matches!(a.ty, 20..=23 | 29 | 30) {
            return None;
        }
        let b = self.d.get(a.at..a.at + a.len)?;
        let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
        Some(String::from_utf8_lossy(&b[..end]).into_owned())
    }

    pub fn int_of(&self, n: usize, name: &str) -> Option<i64> {
        self.int(&self.attr(n, name)?)
    }

    pub fn string_of(&self, n: usize, name: &str) -> Option<String> {
        self.string(&self.attr(n, name)?)
    }
}

/// The `fields::Kind` that edits a value of this LSF type in place, if it's a plain number.
pub fn kind(ty: u32) -> Option<super::fields::Kind> {
    use super::fields::Kind;
    Some(match ty {
        1 => Kind::U8,
        2 => Kind::I16,
        3 => Kind::U16,
        4 => Kind::I32,
        5 => Kind::U32,
        _ => return None,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Builds an LSF file (version 3, attributes in the 12-byte form, as DOS2 saves are)
    /// from nodes (name, parent, attributes (name, type, raw value)), packed with LZ4.
    pub fn build(nodes: &[(&str, i32, Vec<(&str, u32, Vec<u8>)>)], packed: bool) -> Vec<u8> {
        let mut names: Vec<String> = vec![];
        let mut r = |s: &str| -> u32 {
            match names.iter().position(|n| n == s) {
                Some(i) => i as u32,
                None => {
                    names.push(s.to_string());
                    names.len() as u32 - 1
                }
            }
        };
        let (mut nb, mut ab, mut vb) = (vec![], vec![], vec![]);
        let mut ai = 0i32;
        for (i, (name, parent, attrs)) in nodes.iter().enumerate() {
            nb.extend(r(name).to_le_bytes());
            nb.extend((if attrs.is_empty() { -1 } else { ai }).to_le_bytes());
            nb.extend(parent.to_le_bytes());
            for (an, ty, v) in attrs {
                ab.extend(r(an).to_le_bytes());
                ab.extend((ty | (v.len() as u32) << 6).to_le_bytes());
                ab.extend((i as i32).to_le_bytes());
                vb.extend(v);
                ai += 1;
            }
        }
        // One bucket per name, so a reference is (index << 16).
        let mut sn = (names.len() as u32).to_le_bytes().to_vec();
        for n in &names {
            sn.extend(1u16.to_le_bytes());
            sn.extend((n.len() as u16).to_le_bytes());
            sn.extend(n.as_bytes());
        }
        for b in nb.chunks_mut(12) {
            let r = u32::from_le_bytes(b[..4].try_into().unwrap());
            b[..4].copy_from_slice(&(r << 16).to_le_bytes());
        }
        for b in ab.chunks_mut(12) {
            let r = u32::from_le_bytes(b[..4].try_into().unwrap());
            b[..4].copy_from_slice(&(r << 16).to_le_bytes());
        }
        let mut out = b"LSOF".to_vec();
        out.extend(3u32.to_le_bytes());
        out.extend(0x3609_0000u32.to_le_bytes());
        let flags = if packed { 0x22 } else { 0 };
        let mut body = vec![];
        for (i, s) in [&sn, &nb, &ab, &vb].into_iter().enumerate() {
            let c = if packed {
                lspk::compress(s, flags, i != 0).unwrap()
            } else {
                vec![]
            };
            out.extend((s.len() as u32).to_le_bytes());
            out.extend((c.len() as u32).to_le_bytes());
            body.extend(if packed { c } else { s.clone() });
        }
        out.extend([flags, 0, 0, 0]);
        out.extend(0u32.to_le_bytes());
        out.extend(body);
        out
    }

    #[test]
    fn unpacks_reads_and_packs() {
        let f = build(
            &[
                ("Root", -1, vec![("Count", 4, 7i32.to_le_bytes().to_vec())]),
                (
                    "Child",
                    0,
                    vec![
                        ("Name", 22, b"Ann\0".to_vec()),
                        ("Gold", 5, 300u32.to_le_bytes().to_vec()),
                    ],
                ),
                ("Child", 0, vec![("Name", 22, b"Bo\0".to_vec())]),
            ],
            true,
        );
        let plain = unpack(&f).unwrap();
        let l = Lsf::read(&plain).unwrap();
        assert!(Lsf::read(&f).is_none(), "a packed file isn't read directly");
        let root = l.at_path(&["Root"])[0];
        assert_eq!(l.int_of(root, "Count"), Some(7));
        let kids = l.at_path(&["Root", "Child"]);
        assert_eq!(kids.len(), 2);
        assert_eq!(l.string_of(kids[0], "Name").as_deref(), Some("Ann"));
        assert_eq!(l.string_of(kids[1], "Name").as_deref(), Some("Bo"));
        let gold = l.attr(kids[0], "Gold").unwrap();
        assert_eq!(l.int(&gold), Some(300));
        // Unchanged: the original bytes come back.
        assert_eq!(pack(&plain, &f).unwrap(), f);
        // Changed in place: only the values section is packed again.
        let mut edited = plain.clone();
        edited[gold.at..gold.at + 4].copy_from_slice(&301u32.to_le_bytes());
        let out = pack(&edited, &f).unwrap();
        let again = unpack(&out).unwrap();
        assert_eq!(again, edited);
        let l = Lsf::read(&again).unwrap();
        assert_eq!(
            l.int_of(l.at_path(&["Root", "Child"])[0], "Gold"),
            Some(301)
        );
        // A stored (never packed) file reads the same way.
        let raw = build(
            &[("Root", -1, vec![("Count", 4, 1i32.to_le_bytes().to_vec())])],
            false,
        );
        let u = unpack(&raw).unwrap();
        assert_eq!(u, raw);
        let l = Lsf::read(&u).unwrap();
        assert_eq!(l.int_of(0, "Count"), Some(1));
    }
}
