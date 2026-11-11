//! Wasteland 3 saves (`.xml`, which despite the name isn't plain XML).
//!
//! The file is a text header — `XLZF`, then `Key:=Value` lines (`Version`, `Location`,
//! `DataSize` = unpacked size, `SaveDataSize` = packed size, `Hash`, `Levels`…) — followed
//! directly by a raw LibLZF stream that unpacks to one XML document, `<save>…</save>`.
//! The outer part is a layer (`open` / `pack`); the XML is read here by `parse`, which
//! finds the values worth editing and changes their text in place, so everything else in
//! the document is written back byte for byte.
//!
//! Format facts from nharrer/wasteland-3-savegame-decoder and xeus2001/wl3 (both MIT),
//! checked against a sample save. The LZF code is written from the LibLZF format.

use crate::tree::{esc, int_in, unique_key, Backend, Patches, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;

// ------------------------------------------------------------------ LZF

/// Unpacks a raw LibLZF stream; `size` is the expected unpacked size.
pub fn lzf_decompress(src: &[u8], size: usize) -> Result<Vec<u8>, String> {
    let mut out: Vec<u8> = Vec::with_capacity(size);
    let mut i = 0;
    while i < src.len() {
        let ctrl = src[i] as usize;
        i += 1;
        if ctrl < 32 {
            let n = ctrl + 1;
            let lit = src.get(i..i + n).ok_or("LZF data ends inside a literal run")?;
            out.extend_from_slice(lit);
            i += n;
        } else {
            let mut n = ctrl >> 5;
            if n == 7 {
                n += *src.get(i).ok_or("LZF data ends inside a match")? as usize;
                i += 1;
            }
            let lo = *src.get(i).ok_or("LZF data ends inside a match")? as usize;
            i += 1;
            let dist = ((ctrl & 31) << 8 | lo) + 1;
            if dist > out.len() {
                return Err("LZF back-reference before the start of the data".into());
            }
            let mut at = out.len() - dist;
            for _ in 0..n + 2 {
                let b = out[at];
                out.push(b);
                at += 1;
            }
        }
        if out.len() > size {
            return Err("LZF data unpacks to more than the header says".into());
        }
    }
    if out.len() != size {
        return Err(format!("LZF data unpacks to {} bytes, the header says {size}", out.len()));
    }
    Ok(out)
}

/// Packs data as a raw LibLZF stream (greedy matching on a hash of three bytes).
pub fn lzf_compress(d: &[u8]) -> Vec<u8> {
    const HBITS: u32 = 16;
    const MAX_OFF: usize = 1 << 13;
    const MAX_REF: usize = (1 << 8) + (1 << 3); // 264
    fn literals(out: &mut Vec<u8>, lit: &[u8]) {
        for chunk in lit.chunks(32) {
            out.push((chunk.len() - 1) as u8);
            out.extend_from_slice(chunk);
        }
    }
    let mut out = Vec::with_capacity(d.len() / 2 + 16);
    let mut table = vec![usize::MAX; 1 << HBITS];
    let hash = |i: usize| -> usize {
        let v = (u32::from(d[i]) << 16) | (u32::from(d[i + 1]) << 8) | u32::from(d[i + 2]);
        (v.wrapping_mul(2_654_435_761) >> (32 - HBITS)) as usize
    };
    let (mut i, mut lit_start) = (0usize, 0usize);
    while i + 2 < d.len() {
        let h = hash(i);
        let cand = table[h];
        table[h] = i;
        if cand != usize::MAX && i - cand <= MAX_OFF && d[cand..cand + 3] == d[i..i + 3] {
            let max = MAX_REF.min(d.len() - i);
            let mut len = 3;
            while len < max && d[cand + len] == d[i + len] {
                len += 1;
            }
            literals(&mut out, &d[lit_start..i]);
            let off = i - cand - 1;
            let l = len - 2;
            if l < 7 {
                out.push(((l << 5) | (off >> 8)) as u8);
            } else {
                out.push(((7 << 5) | (off >> 8)) as u8);
                out.push((l - 7) as u8);
            }
            out.push(off as u8);
            // Remember the positions inside the match too (better matches later).
            for j in i + 1..(i + len).min(d.len().saturating_sub(2)) {
                table[hash(j)] = j;
            }
            i += len;
            lit_start = i;
        } else {
            i += 1;
        }
    }
    literals(&mut out, &d[lit_start..]);
    out
}

// ------------------------------------------------------------------ outer layer

/// The header's `Key:=Value` lines, the packed stream's start and its end.
fn header(d: &[u8]) -> Option<(Vec<(String, String)>, usize, usize)> {
    if !d.starts_with(b"XLZF\n") {
        return None;
    }
    let mut p = 5;
    let mut keys = vec![];
    // Keys start with a letter; the packed data starts with a literal-run byte (< 32).
    while d.get(p).is_some_and(|b| b.is_ascii_alphabetic()) {
        let e = p + d[p..].iter().take(4096).position(|&b| b == b'\n')?;
        let line = std::str::from_utf8(&d[p..e]).ok()?;
        let (k, v) = line.split_once(":=")?;
        keys.push((k.to_string(), v.trim_end_matches('\r').to_string()));
        p = e + 1;
    }
    let get = |k: &str| keys.iter().find(|(a, _)| a == k).and_then(|(_, v)| v.trim().parse::<usize>().ok());
    let packed = get("SaveDataSize")?;
    get("DataSize")?;
    let end = p.checked_add(packed)?;
    (end <= d.len()).then_some((keys, p, end))
}

/// Header and unpacked XML of a Wasteland 3 save.
pub fn open(d: &[u8]) -> Option<Vec<u8>> {
    let (keys, start, end) = header(d)?;
    let size = keys.iter().find(|(k, _)| k == "DataSize")?.1.trim().parse().ok()?;
    lzf_decompress(&d[start..end], size).ok()
}

/// The file again around new XML: the original when the XML didn't change; otherwise
/// packed afresh with `DataSize` and `SaveDataSize` updated (the other header lines, in
/// their order, and anything after the packed data are kept).
pub fn pack(original: &[u8], xml: &[u8]) -> Result<Vec<u8>, String> {
    let (_, start, end) = header(original).ok_or("not a Wasteland 3 save")?;
    if open(original).as_deref() == Some(xml) {
        return Ok(original.to_vec());
    }
    let packed = lzf_compress(xml);
    if lzf_decompress(&packed, xml.len()).as_deref() != Ok(xml) {
        return Err("LZF packing didn't read back the same (not saved)".into());
    }
    let mut out = Vec::with_capacity(start + packed.len() + 64);
    out.extend_from_slice(b"XLZF\n");
    for line in original[5..start].split_inclusive(|&b| b == b'\n') {
        let text = String::from_utf8_lossy(line);
        match text.split_once(":=") {
            Some(("DataSize", _)) => out.extend(format!("DataSize:={}\n", xml.len()).bytes()),
            Some(("SaveDataSize", _)) => out.extend(format!("SaveDataSize:={}\n", packed.len()).bytes()),
            _ => out.extend_from_slice(line),
        }
    }
    out.extend(packed);
    out.extend_from_slice(&original[end..]);
    Ok(out)
}

// ------------------------------------------------------------------ the XML document

/// An element: its name, where its content starts and ends, and its child elements.
struct Node {
    name: String,
    cs: usize,
    ce: usize,
    kids: Vec<Node>,
}

impl Node {
    fn kid(&self, name: &str) -> Option<&Node> {
        self.kids.iter().find(|k| k.name == name)
    }
    fn all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> + 'a {
        self.kids.iter().filter(move |k| k.name == name)
    }
}

struct P<'a> {
    d: &'a [u8],
    pos: usize,
}

impl P<'_> {
    fn skip(&mut self, end: &[u8]) -> Result<(), String> {
        let n = self.d[self.pos..].windows(end.len()).position(|w| w == end).ok_or("unterminated XML")?;
        self.pos += n + end.len();
        Ok(())
    }
    fn misc(&mut self) -> Result<(), String> {
        loop {
            while self.d.get(self.pos).is_some_and(u8::is_ascii_whitespace) {
                self.pos += 1;
            }
            let rest = &self.d[self.pos..];
            if rest.starts_with(b"<?") {
                self.skip(b"?>")?;
            } else if rest.starts_with(b"<!--") {
                self.skip(b"-->")?;
            } else {
                return Ok(());
            }
        }
    }
    fn name(&mut self) -> String {
        let s = self.pos;
        while self.d.get(self.pos).is_some_and(|b| !b.is_ascii_whitespace() && !b"/>".contains(b)) {
            self.pos += 1;
        }
        String::from_utf8_lossy(&self.d[s..self.pos]).into_owned()
    }
    fn element(&mut self, depth: usize) -> Result<Node, String> {
        if depth > 256 {
            return Err("XML nested too deep".into());
        }
        if self.d.get(self.pos) != Some(&b'<') {
            return Err(format!("expected '<' at byte {}", self.pos));
        }
        self.pos += 1;
        let name = self.name();
        // Attributes (none in these saves, but skip them safely).
        let mut quote = None;
        loop {
            let b = *self.d.get(self.pos).ok_or("unterminated tag")?;
            self.pos += 1;
            match (quote, b) {
                (Some(q), _) if b == q => quote = None,
                (Some(_), _) => {}
                (None, b'"' | b'\'') => quote = Some(b),
                (None, b'>') => break,
                _ => {}
            }
        }
        if self.d[self.pos - 2] == b'/' {
            return Ok(Node { name, cs: self.pos, ce: self.pos, kids: vec![] });
        }
        let cs = self.pos;
        let mut kids = vec![];
        loop {
            let n = self.d[self.pos..].iter().position(|&b| b == b'<').ok_or_else(|| format!("unclosed <{name}>"))?;
            self.pos += n;
            let rest = &self.d[self.pos..];
            if rest.starts_with(b"</") {
                let ce = self.pos;
                self.pos += 2;
                let end = self.name();
                if end != name {
                    return Err(format!("</{end}> doesn't match <{name}>"));
                }
                self.skip(b">")?;
                return Ok(Node { name, cs, ce, kids });
            } else if rest.starts_with(b"<!--") {
                self.skip(b"-->")?;
            } else if rest.starts_with(b"<![CDATA[") {
                self.skip(b"]]>")?;
            } else if rest.starts_with(b"<?") {
                self.skip(b"?>")?;
            } else {
                kids.push(self.element(depth + 1)?);
            }
        }
    }
}

fn root(d: &[u8]) -> Result<Node, String> {
    let mut p = P { d, pos: 0 };
    p.misc()?;
    p.element(0)
}

/// An unpacked Wasteland 3 save: XML whose root is `<save>` with the party in `<pcs>`.
pub fn looks(d: &[u8]) -> bool {
    let head = &d[..d.len().min(4096)];
    d.starts_with(b"<save>") && head.windows(9).any(|w| w == b"<version>") && d.windows(5).any(|w| w == b"<pcs>")
}

/// Most a whole-number field accepts (the game stores them as 32-bit integers).
const MAX: i128 = i32::MAX as i128;

#[derive(Clone, Debug)]
struct W3Backend {
    data: Arc<Vec<u8>>,
    /// Text span of each editable number.
    spans: Vec<(usize, usize)>,
}

impl Backend for W3Backend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = Patches::default();
        for (leaf, v) in changes {
            let (s, e) = self.spans[*leaf];
            p.add(s, e, int_in(v, 0, MAX, "This value")?.to_string().into_bytes());
        }
        p.apply(&self.data)
    }

    fn check(&self, _leaf: usize, v: &Value) -> Result<(), String> {
        int_in(v, 0, MAX, "This value").map(|_| ())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

struct Build<'a> {
    d: &'a [u8],
    spans: Vec<(usize, usize)>,
    editable: HashMap<String, usize>,
}

impl Build<'_> {
    fn text(&self, n: &Node) -> String {
        String::from_utf8_lossy(&self.d[n.cs..n.ce]).into_owned()
    }

    /// A whole number in the file, editable at `ptr` (shown as text and read-only when it
    /// isn't a plain non-negative number).
    fn number(&mut self, map: &mut Map<String, Value>, ptr: &str, key: &str, n: &Node) {
        let t = self.text(n);
        // Only plain digits as the game writes them ("0", "57"; not "+5" or "05").
        let plain = n.kids.is_empty() && !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit()) && (t == "0" || !t.starts_with('0'));
        match t.parse::<i64>() {
            Ok(v) if plain && i128::from(v) <= MAX => {
                self.editable.insert(format!("{ptr}/{}", esc(key)), self.spans.len());
                self.spans.push((n.cs, n.ce));
                map.insert(key.to_string(), Value::from(v));
            }
            _ => {
                map.insert(key.to_string(), Value::String(t));
            }
        }
    }

    /// `<item>` entries of `list` as "template name" → quantity.
    fn items(&mut self, list: &Node, ptr: &str) -> Map<String, Value> {
        let mut m = Map::new();
        for it in list.all("item") {
            let (Some(name), Some(q)) = (it.kid("templateName"), it.kid("quantity")) else { continue };
            let key = unique_key(&m, &self.text(name));
            self.number(&mut m, ptr, &key, q);
        }
        m
    }

    /// One ranger (`<pc>`).
    fn ranger(&mut self, pc: &Node, ptr: &str) -> Map<String, Value> {
        let mut m = Map::new();
        if let Some(c) = pc.kid("companionId") {
            m.insert("Companion id".into(), Value::String(self.text(c)));
        }
        for (label, tag) in RANGER_FIELDS {
            if let Some(n) = pc.kid(tag) {
                self.number(&mut m, ptr, label, n);
            }
        }
        if let Some(skills) = pc.kid("skills") {
            let sp = format!("{ptr}/Skills");
            let mut s = Map::new();
            for sk in skills.all("skill") {
                let (Some(id), Some(lv)) = (sk.kid("skillId"), sk.kid("level")) else { continue };
                let key = unique_key(&s, &format!("Skill {}", self.text(id)));
                self.number(&mut s, &sp, &key, lv);
            }
            m.insert("Skills".into(), Value::Object(s));
        }
        if let Some(perks) = pc.kid("perks") {
            let names: Vec<Value> = perks.all("perk").filter_map(|p| p.kid("perkname")).map(|n| Value::String(self.text(n))).collect();
            m.insert("Perks".into(), Value::Array(names));
        }
        if let Some(eq) = pc.kid("equipment") {
            let items = self.items(eq, &format!("{ptr}/Equipment"));
            m.insert("Equipment".into(), Value::Object(items));
        }
        m
    }
}

/// (label, element) of each ranger's numbers, in the order shown.
pub const RANGER_FIELDS: [(&str, &str); 13] = [
    ("Level", "level"),
    ("XP", "xp"),
    ("Hit points", "hitpoints"),
    ("Coordination", "coordination"),
    ("Luck", "luck"),
    ("Awareness", "awareness"),
    ("Strength", "strength"),
    ("Speed", "speed"),
    ("Intelligence", "intelligence"),
    ("Charisma", "charisma"),
    ("Attribute points", "availableAttributePoints"),
    ("Skill points", "availableSkillPoints"),
    ("Perk points", "perkPoints"),
];

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let save = root(d)?;
    if save.name != "save" {
        return Err("not a Wasteland 3 save".into());
    }
    let pcs = save.kid("pcs").ok_or("no party in this save")?;
    let mut b = Build { d, spans: vec![], editable: HashMap::new() };
    let mut view = Map::new();

    let mut info = Map::new();
    for (label, tag) in [("Version", "version"), ("Location", "currentScene"), ("Saved", "saveTime"), ("Play time", "gameplaytime")] {
        if let Some(n) = save.kid(tag) {
            info.insert(label.into(), Value::String(b.text(n)));
        }
    }
    view.insert("Save".into(), Value::Object(info));

    let mut party = Map::new();
    if let Some(n) = save.kid("money") {
        b.number(&mut party, "/Party", "Money", n);
    }
    view.insert("Party".into(), Value::Object(party));

    for (section, list) in [("Rangers", Some(pcs)), ("Bench", save.kid("rosterpcs"))] {
        let Some(list) = list else { continue };
        let mut m = Map::new();
        for pc in list.all("pc") {
            let name = pc.kid("displayName").map(|n| b.text(n)).filter(|s| !s.trim().is_empty()).unwrap_or_else(|| "Ranger".into());
            let key = unique_key(&m, &name);
            let r = b.ranger(pc, &format!("/{section}/{}", esc(&key)));
            m.insert(key, Value::Object(r));
        }
        if section == "Rangers" || !m.is_empty() {
            view.insert(section.into(), Value::Object(m));
        }
    }

    for (section, tag) in [("Inventory", "hostInventory"), ("Party stash", "partyStash")] {
        if let Some(list) = save.kid(tag) {
            let items = b.items(list, &format!("/{}", esc(section)));
            view.insert(section.into(), Value::Object(items));
        }
    }

    let backend = W3Backend { data: Arc::new(d.to_vec()), spans: b.spans };
    Ok(TreeDoc::new(Value::Object(view), b.editable, "Wasteland 3 save", Box::new(backend)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_xml() -> Vec<u8> {
        let item = |t: &str, q: u32| format!("<item><templateName>{t}</templateName><slot>0</slot><ammoLoaded>0</ammoLoaded><quantity>{q}</quantity><uid>00000000-0000-0000-0000-000000000000</uid></item>");
        let pc = |name: &str, lv: u32| {
            format!(
                "<pc><displayName>{name}</displayName><level>{lv}</level><xp>120</xp><hitpoints>40</hitpoints><companionId>-1</companionId>\
                 <coordination>4</coordination><luck>2</luck><awareness>3</awareness><strength>5</strength><speed>1</speed><intelligence>6</intelligence><charisma>2</charisma>\
                 <availableAttributePoints>0</availableAttributePoints><availableSkillPoints>1</availableSkillPoints>\
                 <equipment>{}</equipment><skills><skill><skillId>10</skillId><level>2</level></skill><skill><skillId>320</skillId><level>1</level></skill></skills>\
                 <levels><level><name>map</name></level></levels><perks><perk><perkname>QRK_None</perkname></perk></perks><perkPoints>1</perkPoints></pc>",
                item("Weapon_Test", 1)
            )
        };
        format!(
            "<save><version>0.91</version><currentScene>test_scene</currentScene><money>250</money><levels><level><name>a</name></level></levels>\
             <pcs>{}{}</pcs><rosterpcs></rosterpcs><hostInventory>{}{}{}</hostInventory><partyStash></partyStash></save>",
            pc("Alpha", 3),
            pc("Bravo", 2),
            item("Ammo9mm", 57),
            item("Ammo9mm", 3),
            item("Scrap_Test", 12)
        )
        .into_bytes()
    }

    fn file(xml: &[u8]) -> Vec<u8> {
        let packed = lzf_compress(xml);
        let mut f = format!("XLZF\nVersion:=0.91\nLocation:=test_scene\nDataSize:={}\nSaveDataSize:={}\nHash:=\nLevels:=3|2\nDLCReq:=0\n", xml.len(), packed.len()).into_bytes();
        f.extend(packed);
        f
    }

    #[test]
    fn lzf_round_trips() {
        let mut data: Vec<u8> = (0..20_000u32).map(|i| (i * 7 % 251) as u8).collect();
        data.extend(std::iter::repeat(b'x').take(5000));
        data.extend(synthetic_xml());
        data.extend((0..3000u32).map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8));
        for d in [&data[..], b"", b"a", b"ab", b"abcabcabcabcabc"] {
            let c = lzf_compress(d);
            assert_eq!(lzf_decompress(&c, d.len()).unwrap(), d);
        }
        assert!(lzf_compress(&data).len() < data.len());
        // Bad streams are refused, not panicking.
        assert!(lzf_decompress(&[0x20, 0], 3).is_err());
        assert!(lzf_decompress(&[5, 1], 6).is_err());
    }

    #[test]
    fn reads_and_edits_a_synthetic_save() {
        let xml = synthetic_xml();
        let f = file(&xml);
        assert_eq!(open(&f).unwrap(), xml);
        // Unchanged XML gives the same file back.
        assert_eq!(pack(&f, &xml).unwrap(), f);
        assert!(looks(&xml));
        let mut doc = parse(&xml).unwrap();
        assert_eq!(doc.kind_name, "Wasteland 3 save");
        let v = &doc.view;
        assert_eq!(v.pointer("/Party/Money"), Some(&Value::from(250)));
        assert_eq!(v.pointer("/Rangers/Alpha/Level"), Some(&Value::from(3)));
        assert_eq!(v.pointer("/Rangers/Bravo/Skills/Skill 320"), Some(&Value::from(1)));
        assert_eq!(v.pointer("/Rangers/Alpha/Perk points"), Some(&Value::from(1)));
        assert_eq!(v.pointer("/Rangers/Alpha/Equipment/Weapon_Test"), Some(&Value::from(1)));
        assert_eq!(v.pointer("/Inventory/Ammo9mm (2)"), Some(&Value::from(3)));
        assert!(doc.set("/Party/Money", Value::from(-1)).is_err());
        doc.set("/Party/Money", Value::from(1_000_000)).unwrap();
        doc.set("/Rangers/Bravo/Strength", Value::from(10)).unwrap();
        doc.set("/Rangers/Alpha/Skills/Skill 10", Value::from(7)).unwrap();
        doc.set("/Inventory/Scrap_Test", Value::from(99)).unwrap();
        let out = doc.to_bytes().unwrap();
        let s = String::from_utf8(out.clone()).unwrap();
        assert!(s.contains("<money>1000000</money>"));
        assert!(s.contains("<skillId>10</skillId><level>7</level>"));
        // Only the edited numbers changed.
        let again = parse(&out).unwrap();
        assert_eq!(again.view, doc.view);
        // Packed again: sizes in the header match, and it unpacks to the edited XML.
        let f2 = pack(&f, &out).unwrap();
        let (keys, start, end) = header(&f2).unwrap();
        let get = |k: &str| keys.iter().find(|(a, _)| a == k).unwrap().1.clone();
        assert_eq!(get("DataSize"), out.len().to_string());
        assert_eq!(get("SaveDataSize"), (end - start).to_string());
        assert_eq!(get("Hash"), "");
        assert_eq!(keys.iter().map(|k| k.0.as_str()).collect::<Vec<_>>(), ["Version", "Location", "DataSize", "SaveDataSize", "Hash", "Levels", "DLCReq"]);
        assert_eq!(open(&f2).unwrap(), out);
    }

    #[test]
    fn opens_through_the_layer_stack() {
        let f = file(&synthetic_xml());
        let d = crate::formats::decode(&f);
        let crate::formats::Content::Tree(t) = &d.content else { panic!("not a tree: {}", d.content.kind()) };
        assert_eq!(t.kind_name, "Wasteland 3 save");
        assert_eq!(crate::formats::encode(&d.layers, &d.content).unwrap(), f);
    }

    /// W3_SAVES = sample saves separated by `|`: each opens, writes back unchanged, and an
    /// edit (money + 1, a skill + 1) reads back with everything else the same.
    #[test]
    #[ignore]
    fn real_saves() {
        let list = std::env::var("W3_SAVES").expect("set W3_SAVES");
        for path in list.split('|').filter(|s| !s.is_empty()) {
            let raw = std::fs::read(path).unwrap();
            let xml = open(&raw).expect("unpacks");
            let again = lzf_compress(&xml);
            assert_eq!(lzf_decompress(&again, xml.len()).unwrap(), xml, "{path}: repack");
            let d = crate::formats::decode(&raw);
            let crate::formats::Content::Tree(t) = &d.content else { panic!("{path}: not a tree") };
            assert_eq!(t.kind_name, "Wasteland 3 save");
            assert_eq!(crate::formats::encode(&d.layers, &d.content).unwrap(), raw, "{path}: unchanged save differs");
            let mut doc = (**t).clone();
            let money = doc.view.pointer("/Party/Money").and_then(Value::as_i64).expect("money");
            doc.set("/Party/Money", Value::from(money + 1)).unwrap();
            let ranger = doc.view["Rangers"].as_object().unwrap().keys().next().unwrap().clone();
            let skills = doc.view["Rangers"][&ranger]["Skills"].as_object().unwrap().clone();
            let (sk, lv) = skills.iter().next().unwrap();
            let sp = format!("/Rangers/{}/Skills/{}", esc(&ranger), esc(sk));
            doc.set(&sp, Value::from(lv.as_i64().unwrap() + 1)).unwrap();
            let content = crate::formats::Content::Tree(Box::new(doc.clone()));
            let out = crate::formats::encode(&d.layers, &content).unwrap();
            let back = crate::formats::decode(&out);
            let crate::formats::Content::Tree(t2) = &back.content else { panic!("{path}: edited save doesn't read") };
            assert_eq!(t2.view, doc.view, "{path}: edited save reads back differently");
            let x1 = open(&raw).unwrap();
            let x2 = open(&out).unwrap();
            let diff = x1.len().abs_diff(x2.len());
            assert!(diff <= 2, "{path}: XML changed by more than the edits");
            println!("{path}: {} rangers, money {money} -> {}, {sk} of {ranger} {lv} -> {}, packed {} -> {} bytes", doc.view["Rangers"].as_object().unwrap().len(), money + 1, lv.as_i64().unwrap() + 1, raw.len(), out.len());
        }
    }
}
