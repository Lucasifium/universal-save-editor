//! The Sims 4 saves (`Slot_0000000N.save`): a DBPF 2.1 package whose one resource of type `0x0D`
//! holds the whole save as a protobuf message, packed with RefPack.
//!
//! The package has a 96-byte header (index entry count at 36, index size at 44, index offset at
//! 64) and a 32-byte-entry index at its end: type, group, instance (two words), offset, size
//! (bit 31 set), unpacked size, compression (0 none, 0xFFFF RefPack), committed. Inside the
//! save message, field 5 (repeated) is a household: id (field 2, fixed 64-bit), name (3) and
//! the funds (5); field 6 (repeated) is a Sim: id (1), household id (4), first name (5), last
//! name (6), gender (7: 4096 male, 8192 female) and age (8: bit flags). Field numbers read
//! from a public save; the funds match the known starting values (20,000 and so on).
//!
//! The editor shows a flat form (`S4FL`) of households and their Sims; on save the funds are
//! put back into the protobuf (every length around them is written again) and the save is
//! stored uncompressed in the package. Sim skills aren't mapped: their statistics are keyed by
//! 64-bit name hashes.

use super::dbpf::qfs;
use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::collections::HashMap;

pub const KIND: &str = "The Sims 4 save";
const HEAD: usize = 96;
const SAVE: u32 = 0x0D;
const FLAT: &[u8; 4] = b"S4FL";

fn u32_at(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

struct Entry {
    at: usize,
    ty: u32,
    offset: usize,
    size: usize,
    mem: usize,
    comp: u16,
}

fn index(d: &[u8]) -> Option<(Vec<Entry>, usize)> {
    if d.len() < HEAD || &d[..4] != b"DBPF" || u32_at(d, 4)? != 2 || u32_at(d, 8)? != 1 {
        return None;
    }
    let (n, size, off) = (u32_at(d, 36)? as usize, u32_at(d, 44)? as usize, u32_at(d, 64)? as usize);
    // Only the plain index (no shared fields) is handled.
    if u32_at(d, off)? != 0 || size != 4 + 32 * n || off.checked_add(size)? > d.len() {
        return None;
    }
    let mut out = vec![];
    for i in 0..n {
        let b = off + 4 + 32 * i;
        out.push(Entry {
            at: b,
            ty: u32_at(d, b)?,
            offset: u32_at(d, b + 16)? as usize,
            size: (u32_at(d, b + 20)? & 0x7FFF_FFFF) as usize,
            mem: u32_at(d, b + 24)? as usize,
            comp: u16::from_le_bytes(d.get(b + 28..b + 30)?.try_into().ok()?),
        });
    }
    Some((out, off))
}

fn save_bytes(d: &[u8], e: &Entry) -> Option<Vec<u8>> {
    let raw = d.get(e.offset..e.offset.checked_add(e.size)?)?;
    match e.comp {
        0 => Some(raw.to_vec()),
        0xFFFF => qfs(raw),
        _ => None,
    }
}

// ---- protobuf helpers

fn varint(b: &[u8], p: &mut usize) -> Option<u64> {
    let (mut r, mut s) = (0u64, 0);
    loop {
        let c = *b.get(*p)?;
        *p += 1;
        r |= ((c & 0x7F) as u64).checked_shl(s)?;
        if c < 0x80 {
            return Some(r);
        }
        s += 7;
        if s > 63 {
            return None;
        }
    }
}

fn put_varint(out: &mut Vec<u8>, mut v: u64) {
    while v >= 0x80 {
        out.push((v as u8 & 0x7F) | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

/// One protobuf field: number, wire type, the value (varint or fixed bytes or the payload).
struct Field {
    num: u32,
    wire: u8,
    varint: u64,
    bytes: Vec<u8>,
}

fn fields(b: &[u8]) -> Option<Vec<Field>> {
    let mut p = 0;
    let mut out = vec![];
    while p < b.len() {
        let k = varint(b, &mut p)?;
        let (num, wire) = ((k >> 3) as u32, (k & 7) as u8);
        let mut f = Field { num, wire, varint: 0, bytes: vec![] };
        match wire {
            0 => f.varint = varint(b, &mut p)?,
            1 => {
                f.bytes = b.get(p..p + 8)?.to_vec();
                p += 8;
            }
            5 => {
                f.bytes = b.get(p..p + 4)?.to_vec();
                p += 4;
            }
            2 => {
                let l = varint(b, &mut p)? as usize;
                f.bytes = b.get(p..p.checked_add(l)?)?.to_vec();
                p += l;
            }
            _ => return None,
        }
        out.push(f);
    }
    Some(out)
}

fn encode(f: &Field, out: &mut Vec<u8>) {
    put_varint(out, (f.num as u64) << 3 | f.wire as u64);
    match f.wire {
        0 => put_varint(out, f.varint),
        2 => {
            put_varint(out, f.bytes.len() as u64);
            out.extend(&f.bytes);
        }
        _ => out.extend(&f.bytes),
    }
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

fn get(fs: &[Field], n: u32) -> Option<&Field> {
    fs.iter().find(|f| f.num == n)
}

// ---- the flat form

pub fn looks_package(d: &[u8]) -> bool {
    index(d).is_some_and(|(es, _)| es.iter().any(|e| e.ty == SAVE && e.mem > 1000))
}

pub fn flatten(d: &[u8]) -> Option<Vec<u8>> {
    let (es, _) = index(d)?;
    let e = es.iter().find(|e| e.ty == SAVE)?;
    let top = fields(&save_bytes(d, e)?)?;
    let mut out = FLAT.to_vec();
    let hh: Vec<Vec<Field>> = top.iter().filter(|f| f.num == 5 && f.wire == 2).filter_map(|f| fields(&f.bytes)).collect();
    out.extend((hh.len() as u32).to_le_bytes());
    for h in &hh {
        out.extend((get(h, 5).map_or(0, |f| f.varint) as u32).to_le_bytes());
        out.extend(get(h, 2).map_or(vec![0; 8], |f| f.bytes.clone()));
        let name = get(h, 3).map_or(vec![], |f| f.bytes.clone());
        out.extend((name.len() as u16).to_le_bytes());
        out.extend(name);
    }
    let sims: Vec<Vec<Field>> = top.iter().filter(|f| f.num == 6 && f.wire == 2).filter_map(|f| fields(&f.bytes)).collect();
    out.extend((sims.len() as u32).to_le_bytes());
    for s in &sims {
        out.extend(get(s, 4).map_or(vec![0; 8], |f| f.bytes.clone()));
        out.extend((get(s, 8).map_or(0, |f| f.varint) as u32).to_le_bytes());
        out.extend((get(s, 7).map_or(0, |f| f.varint) as u32).to_le_bytes());
        for n in [5, 6] {
            let t = get(s, n).map_or(vec![], |f| f.bytes.clone());
            out.extend((t.len() as u16).to_le_bytes());
            out.extend(t);
        }
    }
    Some(out)
}

pub fn looks(d: &[u8]) -> bool {
    d.starts_with(FLAT) && d.len() >= 12
}

fn age(v: u32) -> String {
    match v {
        1 => "Baby".into(),
        2 => "Toddler".into(),
        4 => "Child".into(),
        8 => "Teen".into(),
        16 => "Young adult".into(),
        32 => "Adult".into(),
        64 => "Elder".into(),
        128 => "Infant".into(),
        n => format!("Age {n}"),
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let bad = || "damaged Sims 4 data".to_string();
    let u32a = |o: usize| u32_at(d, o).ok_or_else(bad);
    let u16a = |o: usize| d.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as usize).ok_or_else(bad);
    let mut p = 4;
    let nh = u32a(p)? as usize;
    p += 4;
    let mut hs: Vec<(String, usize, u64)> = vec![];
    for _ in 0..nh {
        let money = p;
        let id = u64::from_le_bytes(d.get(p + 4..p + 12).ok_or_else(bad)?.try_into().unwrap());
        let nl = u16a(p + 12)?;
        let name = text(d.get(p + 14..p + 14 + nl).ok_or_else(bad)?);
        p += 14 + nl;
        hs.push((name, money, id));
    }
    let ns = u32a(p)? as usize;
    p += 4;
    let mut by_house: HashMap<u64, Vec<String>> = HashMap::new();
    for _ in 0..ns {
        let hid = u64::from_le_bytes(d.get(p..p + 8).ok_or_else(bad)?.try_into().unwrap());
        let (a, g) = (u32a(p + 8)?, u32a(p + 12)?);
        let l1 = u16a(p + 16)?;
        let first = text(d.get(p + 18..p + 18 + l1).ok_or_else(bad)?);
        let l2 = u16a(p + 18 + l1)?;
        let last = text(d.get(p + 20 + l1..p + 20 + l1 + l2).ok_or_else(bad)?);
        p += 20 + l1 + l2;
        let who = format!("{first} {last}").trim().to_string();
        by_house.entry(hid).or_default().push(format!("{who} ({}, {})", age(a), if g == 4096 { "male" } else { "female" }));
    }
    let mut b = Builder::new(d);
    let mut used = std::collections::HashSet::new();
    for (i, (name, money, id)) in hs.iter().enumerate() {
        let mut label = if name.is_empty() { format!("Household {}", i + 1) } else { name.clone() };
        if !used.insert(label.clone()) {
            label = format!("{label} ({})", i + 1);
            used.insert(label.clone());
        }
        b.ranged(&["Households", &label, "Funds"], *money, Kind::U32, 0, 99_999_999);
        if let Some(sims) = by_house.get(id) {
            b.info(&["Households", &label, "Sims"], sims.join(", "));
        }
    }
    Ok(b.finish(KIND, Checksum::None).with_note("Household funds are saved back into the game data, which is stored uncompressed in the package (it gets bigger). Close the game first. Sim skills and traits aren't mapped yet."))
}

pub fn rebuild(original: &[u8], flat: &[u8]) -> Result<Vec<u8>, String> {
    let (es, ioff) = index(original).ok_or("the package changed")?;
    let e = es.iter().find(|e| e.ty == SAVE).ok_or("the save data is missing")?;
    let old = flatten(original).ok_or("the save data can't be read")?;
    if old == flat {
        return Ok(original.to_vec());
    }
    // Funds by household, in file order.
    let money = |f: &[u8]| -> Option<Vec<u32>> {
        let mut p = 4;
        let n = u32_at(f, p)? as usize;
        p += 4;
        let mut v = vec![];
        for _ in 0..n {
            v.push(u32_at(f, p)?);
            p += 14 + u16::from_le_bytes(f.get(p + 12..p + 14)?.try_into().ok()?) as usize;
        }
        Some(v)
    };
    let (Some(new), Some(cur)) = (money(flat), money(&old)) else { return Err("the save data is damaged".into()) };
    if new.len() != cur.len() {
        return Err("households can't be added or removed".into());
    }
    let top = fields(&save_bytes(original, e).ok_or("the save data can't be read")?).ok_or("the save data can't be read")?;
    let mut body = vec![];
    let mut k = 0;
    for f in &top {
        if f.num == 5 && f.wire == 2 && fields(&f.bytes).is_some() {
            let mut hf = fields(&f.bytes).unwrap();
            if new[k] != cur[k] {
                if let Some(m) = hf.iter_mut().find(|x| x.num == 5 && x.wire == 0) {
                    m.varint = new[k] as u64;
                }
                let mut inner = vec![];
                for x in &hf {
                    encode(x, &mut inner);
                }
                encode(&Field { num: 5, wire: 2, varint: 0, bytes: inner }, &mut body);
            } else {
                encode(f, &mut body);
            }
            k += 1;
        } else {
            encode(f, &mut body);
        }
    }
    // The package: the header, every resource in order (the save stored uncompressed), the index.
    let mut out = original[..HEAD].to_vec();
    let mut index_bytes = original[ioff..ioff + 4].to_vec();
    for x in &es {
        let bytes: Vec<u8>;
        let (slice, size, mem, comp): (&[u8], usize, usize, u16) = if x.at == e.at {
            bytes = body.clone();
            (&bytes, body.len(), body.len(), 0)
        } else {
            (original.get(x.offset..x.offset + x.size).ok_or("a resource can't be read")?, x.size, x.mem, x.comp)
        };
        let off = out.len();
        out.extend_from_slice(slice);
        let mut ent = original[x.at..x.at + 32].to_vec();
        ent[16..20].copy_from_slice(&(off as u32).to_le_bytes());
        ent[20..24].copy_from_slice(&(size as u32 | 0x8000_0000).to_le_bytes());
        ent[24..28].copy_from_slice(&(mem as u32).to_le_bytes());
        ent[28..30].copy_from_slice(&comp.to_le_bytes());
        index_bytes.extend(ent);
    }
    let at = out.len();
    out.extend(index_bytes);
    out[64..68].copy_from_slice(&(at as u32).to_le_bytes());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn fld(num: u32, wire: u8, v: u64, bytes: &[u8]) -> Vec<u8> {
        let mut o = vec![];
        encode(&Field { num, wire, varint: v, bytes: bytes.to_vec() }, &mut o);
        o
    }

    fn package() -> Vec<u8> {
        let mut hh = fld(2, 1, 0, &7u64.to_le_bytes());
        hh.extend(fld(3, 2, 0, b"Goth"));
        hh.extend(fld(5, 0, 20_000, &[]));
        let mut sim = fld(1, 1, 0, &9u64.to_le_bytes());
        sim.extend(fld(4, 1, 0, &7u64.to_le_bytes()));
        sim.extend(fld(5, 2, 0, b"Bella"));
        sim.extend(fld(6, 2, 0, b"Goth"));
        sim.extend(fld(7, 0, 8192, &[]));
        sim.extend(fld(8, 0, 16, &[]));
        let mut body = fld(5, 2, 0, &hh);
        body.extend(fld(6, 2, 0, &sim));
        body.extend(vec![0u8; 2000]);
        // Padding as an unknown varint field to keep the message valid.
        let mut valid = fld(5, 2, 0, &hh);
        valid.extend(fld(6, 2, 0, &sim));
        for _ in 0..400 {
            valid.extend(fld(99, 0, 1, &[]));
        }
        let mut d = b"DBPF".to_vec();
        d.extend([2, 0, 0, 0, 1, 0, 0, 0]);
        d.resize(HEAD, 0);
        let off = d.len();
        d.extend(&valid);
        let ioff = d.len();
        d.extend(0u32.to_le_bytes());
        let mut ent = vec![0u8; 32];
        ent[0..4].copy_from_slice(&SAVE.to_le_bytes());
        ent[16..20].copy_from_slice(&(off as u32).to_le_bytes());
        ent[20..24].copy_from_slice(&(valid.len() as u32 | 0x8000_0000).to_le_bytes());
        ent[24..28].copy_from_slice(&(valid.len() as u32).to_le_bytes());
        d.extend(ent);
        d[36..40].copy_from_slice(&1u32.to_le_bytes());
        d[44..48].copy_from_slice(&36u32.to_le_bytes());
        d[64..68].copy_from_slice(&(ioff as u32).to_le_bytes());
        d
    }

    #[test]
    fn reads_edits_and_rebuilds() {
        let d = package();
        assert!(looks_package(&d));
        let f = flatten(&d).unwrap();
        let mut doc = parse(&f).unwrap();
        let v = |doc: &TreeDoc, p: &str| doc.view.pointer(p).cloned();
        assert_eq!(v(&doc, "/Households/Goth/Funds"), Some(Value::from(20_000)));
        assert_eq!(v(&doc, "/Households/Goth/Sims"), Some(Value::from("Bella Goth (Young adult, female)")));
        assert_eq!(rebuild(&d, &doc.to_bytes().unwrap()).unwrap(), d);
        doc.set("/Households/Goth/Funds", Value::from(99_999_999)).unwrap();
        let out = rebuild(&d, &doc.to_bytes().unwrap()).unwrap();
        let again = parse(&flatten(&out).unwrap()).unwrap();
        assert_eq!(v(&again, "/Households/Goth/Funds"), Some(Value::from(99_999_999)));
        assert_eq!(v(&again, "/Households/Goth/Sims"), Some(Value::from("Bella Goth (Young adult, female)")));
    }

    /// Research aid: SAVE_EDITOR_SIMS4 = `.save` files separated by `|`.
    #[test]
    #[ignore]
    fn real_files() {
        for path in std::env::var("SAVE_EDITOR_SIMS4").unwrap().split('|').filter(|s| !s.is_empty()) {
            let d = std::fs::read(path).unwrap();
            let f = flatten(&d).unwrap();
            let mut doc = parse(&f).unwrap();
            for (k, v) in doc.view["Households"].as_object().unwrap().iter().take(8) {
                println!("{k}: {} | {}", v["Funds"], v["Sims"]);
            }
            assert_eq!(rebuild(&d, &doc.to_bytes().unwrap()).unwrap(), d);
            let first = doc.view["Households"].as_object().unwrap().keys().next().unwrap().clone();
            doc.set(&format!("/Households/{first}/Funds"), Value::from(1_234_567)).unwrap();
            let out = rebuild(&d, &doc.to_bytes().unwrap()).unwrap();
            let again = parse(&flatten(&out).unwrap()).unwrap();
            assert_eq!(again.view["Households"][&first]["Funds"], Value::from(1_234_567));
            // Every other resource is unchanged.
            let (a, _) = index(&d).unwrap();
            let (b, _) = index(&out).unwrap();
            assert_eq!(a.len(), b.len());
            for (x, y) in a.iter().zip(&b) {
                if x.ty != SAVE {
                    assert_eq!(&d[x.offset..x.offset + x.size], &out[y.offset..y.offset + y.size]);
                }
            }
            println!("rewritten: {} -> {} bytes", d.len(), out.len());
        }
    }
}
