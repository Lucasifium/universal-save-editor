//! Osiris story saves (`StorySave.bin` in a Baldur's Gate 3 save; `story.dat` in the Divinity
//! games): the scripting engine's state. The file has no offset table, so it is read section
//! by section: header, types, enums, objects, functions, the rule network (nodes, adapters),
//! then the databases, whose facts hold the story state (flags, approval, counters, quests).
//! Goals and the actions after them aren't needed and aren't read.
//!
//! Layout as in LSLib's Osiris reader (Norbyte, MIT): `LSLib/LS/Story` (`Story.cs`,
//! `Common.cs`, `Value.cs`, `Node.cs`, `Join.cs`, `Rel.cs`, `RelOp.cs`, `Rule.cs`, `Call.cs`,
//! `Adapter.cs`, `Database.cs`, `Function.cs`). Strings after the header are XOR-ed with 0xAD
//! from version 1.4 on.

use std::collections::HashMap;

const VER_ADD_VERSION_STRING: u32 = 0x0102;
const VER_ADD_DEBUG_FLAGS: u32 = 0x0103;
const VER_SCRAMBLE: u32 = 0x0104;
const VER_ADD_TYPE_MAP: u32 = 0x0105;
const VER_ADD_QUERY: u32 = 0x0106;
const VER_TYPE_ALIASES: u32 = 0x0109;
const VER_ENHANCED_TYPES: u32 = 0x010a;
const VER_EXTERNAL_STRING_TABLE: u32 = 0x010b;
const VER_REMOVE_EXTERNAL_STRING_TABLE: u32 = 0x010c;
const VER_ENUMS: u32 = 0x010d;
const VER_VALUE_FLAGS: u32 = 0x010e;
const VER_LAST: u32 = 0x010f;

/// One value of a fact.
#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    Int { at: usize, v: i32 },
    Int64 { at: usize, v: i64 },
    Real { at: usize, v: f32 },
    Str(String),
    None,
}

/// One row of a database: its values and the byte range it takes in the file.
#[derive(Clone, Debug)]
pub struct Fact {
    pub start: usize,
    pub end: usize,
    pub values: Vec<Val>,
}

#[derive(Clone, Debug)]
pub struct Database {
    #[allow(dead_code)]
    pub index: u32,
    pub name: String,
    /// Where the fact count (u32) is.
    pub count_at: usize,
    pub facts: Vec<Fact>,
}

pub struct Story {
    #[allow(dead_code)]
    pub version: u32,
    pub databases: Vec<Database>,
}

impl Story {
    pub fn db(&self, name: &str) -> Option<&Database> {
        self.databases.iter().find(|d| d.name == name)
    }
}

struct R<'a> {
    d: &'a [u8],
    p: usize,
    ver: u32,
    scramble: u8,
    short_ids: Option<bool>,
    aliases: HashMap<u32, u32>,
    enums: HashMap<u32, ()>,
}

impl<'a> R<'a> {
    fn bytes(&mut self, n: usize) -> Option<&'a [u8]> {
        let b = self.d.get(self.p..self.p.checked_add(n)?)?;
        self.p += n;
        Some(b)
    }
    fn u8(&mut self) -> Option<u8> {
        Some(self.bytes(1)?[0])
    }
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.bytes(2)?.try_into().ok()?))
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.bytes(4)?.try_into().ok()?))
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.bytes(8)?.try_into().ok()?))
    }
    fn string(&mut self) -> Option<String> {
        let start = self.p;
        let rel = self.d.get(start..)?.iter().position(|&b| b == self.scramble)?;
        let raw: Vec<u8> = self.d[start..start + rel].iter().map(|b| b ^ self.scramble).collect();
        self.p = start + rel + 1;
        Some(String::from_utf8_lossy(&raw).into_owned())
    }
    fn type_id(&mut self) -> Option<u32> {
        if self.short_ids == Some(true) {
            self.u16().map(u32::from)
        } else {
            self.u32()
        }
    }
    fn count_u32(&mut self) -> Option<usize> {
        let n = self.u32()? as usize;
        // Each item takes at least one byte; a count larger than what's left is a misread.
        (n <= self.d.len() - self.p).then_some(n)
    }

    fn value(&mut self) -> Option<Val> {
        if self.ver >= VER_VALUE_FLAGS {
            self.u8()?; // index
            let flags = self.u8()?;
            if flags & 0x08 == 0 {
                return Some(Val::None);
            }
        }
        match self.u8()? {
            b'1' => {
                self.type_id()?;
                let at = self.p;
                let v = self.u32()? as i32;
                Some(Val::Int { at, v })
            }
            b'0' => {
                let id = self.type_id()?;
                let t = self.aliases.get(&id).copied().unwrap_or(id);
                if self.ver < VER_ENHANCED_TYPES {
                    return None; // Divinity: Original Sin 1 layouts aren't read here.
                }
                match t {
                    0 => Some(Val::None),
                    1 => {
                        let at = self.p;
                        Some(Val::Int { at, v: self.u32()? as i32 })
                    }
                    2 => {
                        let at = self.p;
                        Some(Val::Int64 { at, v: self.u64()? as i64 })
                    }
                    3 => {
                        let at = self.p;
                        Some(Val::Real { at, v: f32::from_le_bytes(self.bytes(4)?.try_into().ok()?) })
                    }
                    4 | 5 => {
                        if self.u8()? > 0 {
                            Some(Val::Str(self.string()?))
                        } else {
                            Some(Val::None)
                        }
                    }
                    _ => Some(Val::Str(self.string()?)),
                }
            }
            b'e' => {
                let id = u32::from(self.u16()?);
                self.enums.contains_key(&id).then_some(())?;
                Some(Val::Str(self.string()?))
            }
            _ => None,
        }
    }

    /// TypedValue / Variable: a value, plus flag bytes in versions before value flags.
    fn typed_value(&mut self) -> Option<()> {
        self.value()?;
        if self.ver < VER_VALUE_FLAGS {
            self.bytes(3)?;
        }
        Some(())
    }
    fn variable(&mut self) -> Option<()> {
        self.typed_value()?;
        if self.ver < VER_VALUE_FLAGS {
            self.bytes(3)?;
        }
        Some(())
    }

    fn tuple(&mut self) -> Option<()> {
        let n = self.u8()?;
        for _ in 0..n {
            if self.ver < VER_VALUE_FLAGS {
                self.u8()?;
            }
            self.value()?;
        }
        Some(())
    }

    fn param_list(&mut self) -> Option<Vec<u32>> {
        let n = self.u8()?;
        let mut out = vec![];
        for _ in 0..n {
            if self.short_ids.is_none() {
                let id = u32::from_le_bytes(self.d.get(self.p..self.p + 4)?.try_into().ok()?);
                self.short_ids = Some(id > 0xff);
            }
            out.push(self.type_id()?);
        }
        Some(out)
    }

    fn nei(&mut self) -> Option<()> {
        self.bytes(12).map(|_| ())
    }

    fn call(&mut self) -> Option<()> {
        let name = self.string()?;
        if !name.is_empty() {
            if self.u8()? > 0 {
                let n = self.u8()?;
                for _ in 0..n {
                    if self.ver < VER_VALUE_FLAGS {
                        let t = self.u8()?;
                        if t == 1 {
                            self.variable()?;
                        } else {
                            self.typed_value()?;
                        }
                    } else {
                        self.variable()?;
                    }
                }
            }
            self.u8()?; // negate
        }
        self.u32()?; // goal id or debug hook
        Some(())
    }

    /// A node; returns (database index, name) for database nodes.
    fn node(&mut self) -> Option<Option<(u32, String)>> {
        let ty = self.u8()?;
        self.u32()?; // node id
        let db = self.u32()?;
        let name = self.string()?;
        if !name.is_empty() {
            self.u8()?; // parameter count
        }
        match ty {
            // Database, Proc: the nodes that read them.
            1 | 2 => {
                let n = self.count_u32()?;
                self.bytes(12 * n)?;
            }
            // Queries: nothing more.
            3 | 8 | 9 => {}
            // And, NotAnd: next node, then both sides of the join.
            4 | 5 => {
                self.nei()?;
                self.bytes(16)?;
                self.u32()?;
                self.nei()?;
                self.u8()?;
                self.u32()?;
                self.nei()?;
                self.u8()?;
            }
            // RelOp, Rule: next node, then the single-parent part.
            6 | 7 => {
                self.nei()?;
                self.bytes(12)?;
                self.nei()?;
                self.u8()?;
                if ty == 6 {
                    self.bytes(2)?;
                    self.value()?;
                    self.value()?;
                    self.u32()?;
                } else {
                    let calls = self.count_u32()?;
                    for _ in 0..calls {
                        self.call()?;
                    }
                    let vars = self.u8()?;
                    for _ in 0..vars {
                        if self.ver < VER_VALUE_FLAGS {
                            (self.u8()? == 1).then_some(())?;
                        }
                        self.variable()?;
                    }
                    self.u32()?; // line
                    if self.ver >= VER_ADD_QUERY {
                        self.u8()?;
                    }
                }
            }
            _ => return None,
        }
        Some((ty == 1 && db != 0).then_some((db, name)))
    }
}

/// Reads the story up to and including its databases.
pub fn read(d: &[u8]) -> Option<Story> {
    let mut r = R { d, p: 0, ver: 0, scramble: 0, short_ids: None, aliases: HashMap::new(), enums: HashMap::new() };
    r.u8()?;
    let _version = r.string()?;
    let major = r.u8()?;
    let minor = r.u8()?;
    r.ver = (u32::from(major) << 8) | u32::from(minor);
    if r.ver > VER_LAST || r.ver < VER_ENHANCED_TYPES {
        return None;
    }
    r.bytes(2)?;
    if r.ver >= VER_ADD_VERSION_STRING {
        r.bytes(0x80)?;
    }
    if r.ver >= VER_ADD_DEBUG_FLAGS {
        r.u32()?;
    }
    if r.ver < VER_REMOVE_EXTERNAL_STRING_TABLE {
        r.short_ids = Some(false);
    } else if r.ver >= VER_ENUMS {
        r.short_ids = Some(true);
    }
    if r.ver >= VER_SCRAMBLE {
        r.scramble = 0xAD;
    }
    if r.ver >= VER_ADD_TYPE_MAP {
        let n = r.count_u32()?;
        let mut types: HashMap<u32, u32> = HashMap::new();
        for _ in 0..n {
            r.string()?;
            let idx = u32::from(r.u8()?);
            let alias = if r.ver >= VER_TYPE_ALIASES { u32::from(r.u8()?) } else { 0 };
            types.insert(idx, alias);
        }
        for (&k, &a) in &types {
            if a != 0 {
                let mut id = a;
                let mut guard = 0;
                while id != 0 && types.get(&id).is_some_and(|&x| x != 0) && guard < 64 {
                    id = types[&id];
                    guard += 1;
                }
                r.aliases.insert(k, id);
            }
        }
    }
    if r.ver >= VER_EXTERNAL_STRING_TABLE && r.ver < VER_REMOVE_EXTERNAL_STRING_TABLE {
        let n = r.count_u32()?;
        for _ in 0..n {
            r.string()?;
        }
    }
    if r.ver >= VER_ENUMS {
        let n = r.count_u32()?;
        for _ in 0..n {
            let id = u32::from(r.u16()?);
            let m = r.count_u32()?;
            for _ in 0..m {
                r.string()?;
                r.u64()?;
            }
            r.enums.insert(id, ());
        }
    }
    // Objects.
    let n = r.count_u32()?;
    for _ in 0..n {
        r.string()?;
        r.bytes(17)?;
    }
    // Functions.
    let n = r.count_u32()?;
    for _ in 0..n {
        r.bytes(12)?; // line, condition and action references
        r.u32()?; // node
        r.u8()?; // type
        r.bytes(16)?;
        r.string()?;
        let mask = r.count_u32()?;
        r.bytes(mask)?;
        r.param_list()?;
    }
    // Rule network.
    let mut names: HashMap<u32, String> = HashMap::new();
    let n = r.count_u32()?;
    for _ in 0..n {
        if let Some((db, name)) = r.node()? {
            names.insert(db, name);
        }
    }
    let n = r.count_u32()?;
    for _ in 0..n {
        r.u32()?;
        r.tuple()?;
        let k = r.u8()?;
        r.bytes(usize::from(k))?;
        let k = r.u8()?;
        r.bytes(2 * usize::from(k))?;
    }
    // Databases.
    let n = r.count_u32()?;
    let mut databases = Vec::with_capacity(n);
    for _ in 0..n {
        let index = r.u32()?;
        r.param_list()?;
        let count_at = r.p;
        let facts_n = r.count_u32()?;
        let mut facts = Vec::with_capacity(facts_n.min(1 << 16));
        for _ in 0..facts_n {
            let start = r.p;
            let cols = r.u8()?;
            let mut values = Vec::with_capacity(usize::from(cols));
            for _ in 0..cols {
                values.push(r.value()?);
            }
            facts.push(Fact { start, end: r.p, values });
        }
        let name = names.get(&index).cloned().unwrap_or_default();
        databases.push(Database { index, name, count_at, facts });
    }
    Some(Story { version: r.ver, databases })
}

/// A character reference's display part: "S_Player_ShadowHeart_3ed74f06-…" → "S_Player_ShadowHeart".
pub fn strip_guid(s: &str) -> &str {
    if s.len() > 37 && s.as_bytes()[s.len() - 37] == b'_' && s[s.len() - 36..].chars().filter(|&c| c == '-').count() == 4 {
        &s[..s.len() - 37]
    } else {
        s
    }
}

/// The GUID at the end of a reference, if it has one.
pub fn guid_of(s: &str) -> Option<&str> {
    (strip_guid(s).len() != s.len()).then(|| &s[s.len() - 36..])
}

#[cfg(test)]
mod real {
    /// Research aid: SAVE_EDITOR_LSV = `.lsv` files separated by `|`; prints the story's databases.
    #[test]
    #[ignore]
    fn dump_story() {
        for path in std::env::var("SAVE_EDITOR_LSV").unwrap().split('|').filter(|s| !s.is_empty()) {
            let raw = std::fs::read(path).unwrap();
            let pkg = super::super::lspk::read(&raw).unwrap();
            let e = pkg.find("StorySave.bin").unwrap();
            let d = pkg.file(&raw, e).unwrap();
            let t = std::time::Instant::now();
            let s = super::read(&d).expect("story reads");
            println!("== {path}: v{:x}, {} databases, {} bytes, {:?}", s.version, s.databases.len(), d.len(), t.elapsed());
            let want: Vec<String> = std::env::var("DBS").unwrap_or_default().split(',').map(String::from).collect();
            for db in &s.databases {
                if want.iter().any(|w| !w.is_empty() && db.name.starts_with(w.as_str())) {
                    println!("  {} ({} facts)", db.name, db.facts.len());
                    for f in db.facts.iter().take(std::env::var("N").ok().and_then(|n| n.parse().ok()).unwrap_or(15)) {
                        println!("     {:?}", f.values);
                    }
                }
            }
        }
    }
}
