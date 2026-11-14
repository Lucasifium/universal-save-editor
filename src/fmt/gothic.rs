//! Gothic and Gothic II (`SAVEDAT.SAV` in a save slot): a ZenGin binary archive. The script
//! variables (chapter, quest and story flags, dialogue "already said" flags) are int values that
//! follow their name; each one is opened and edited in place. The character's attributes live in
//! `WORLD.SAV` and are not covered.
//!
//! The reader follows GothicSaveEditor (FlyCrafting / AnImperialMan, MIT).

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::collections::HashMap;

pub const KIND: &str = "Gothic save data";
const VALUE: u8 = 0x12;
const NAME: u8 = 0x01;
const DIALOG: u8 = 0x06;
const NORMAL: u8 = 0x02;

pub fn looks(d: &[u8]) -> bool {
    d.starts_with(b"ZenGin Archive") && d.len() < 8 << 20 && d[..d.len().min(128)].windows(8).any(|w| w == b"BIN_SAFE")
}

struct Var {
    name: String,
    at: usize,
    index: Option<usize>,
}

struct Reader<'a> {
    b: &'a [u8],
    dialog: bool,
    dialog_begin: bool,
    dialog_end: bool,
    mission: bool,
}

impl Reader<'_> {
    fn i32_at(&self, at: usize) -> Option<i32> {
        Some(i32::from_le_bytes(self.b.get(at..at + 4)?.try_into().ok()?))
    }

    /// Reads the values after a name: (positions, values), moving `r` past them.
    fn variable(&mut self, r: &mut usize) -> Option<(Vec<usize>, Vec<i32>)> {
        let mut pos = vec![0usize];
        let mut vals = vec![0i32];
        let mut size = i64::MAX;
        let mut it: i64 = -1;
        while *self.b.get(*r)? == VALUE {
            *r += 5;
            if *self.b.get(*r)? == NAME {
                break;
            }
            *r += 1;
            let marker = *self.b.get(*r - 1)?;
            if marker == NORMAL {
                if self.dialog_begin {
                    self.mission = true;
                    self.dialog = false;
                    self.dialog_end = true;
                    self.dialog_begin = false;
                }
                pos[0] = *r;
                vals[0] = self.i32_at(*r)?;
                if it == -1 {
                    size = i64::from(vals[0]);
                }
                if it >= 0 && it < size {
                    if it == 0 {
                        if size > 1 << 20 {
                            return None;
                        }
                        pos = vec![0; size as usize];
                        vals = vec![0; size as usize];
                    }
                    pos[it as usize] = *r;
                    vals[it as usize] = self.i32_at(*r)?;
                }
                it += 1;
            } else if marker == DIALOG {
                vals[0] = self.i32_at(*r)?;
                pos[0] = *r;
            }
            *r += 4;
            if self.dialog && *self.b.get(*r - 5)? == DIALOG {
                self.dialog_begin = true;
                return Some((pos, vals));
            }
        }
        Some((pos, vals))
    }

    fn name(&mut self, i: &mut usize) -> Option<String> {
        *i += 1;
        let len = i16::from_le_bytes(self.b.get(*i..*i + 2)?.try_into().ok()?);
        *i += 2;
        if len <= 0 {
            return Some(String::new());
        }
        let end = *i + len as usize;
        let s: String = self.b.get(*i..end)?.iter().map(|c| char::from(*c)).collect();
        *i = end - 1;
        if !self.mission {
            return Some(s);
        }
        if s == "[]" {
            self.mission = false;
        }
        Some(String::new())
    }
}

fn add(out: &mut Vec<Var>, name: &str, pos: &[usize]) {
    for (i, p) in pos.iter().enumerate() {
        out.push(Var { name: name.to_string(), at: *p, index: (pos.len() > 1).then_some(i) });
    }
}

fn read(d: &[u8]) -> Option<Vec<Var>> {
    let start = d.windows(8).position(|w| w == [2, 0, 0, 0, 1, 0, 0, 0])? + 8;
    let last = i32::from_le_bytes(d.get(start..start + 4)?.try_into().ok()?);
    if last <= 0 {
        return None;
    }
    let (start, last) = (start + 4, (last as usize).min(d.len()));
    let mut rd = Reader { b: d, dialog: true, dialog_begin: false, dialog_end: false, mission: false };
    let (mut need, mut name, mut cur) = (false, String::new(), (vec![0usize], vec![0i32]));
    let mut out = Vec::new();
    let mut i = start;
    while i < last {
        if d[i] == VALUE {
            if rd.dialog {
                cur = rd.variable(&mut i)?;
            } else if need && !name.trim().is_empty() {
                let (p, _) = rd.variable(&mut i)?;
                if rd.dialog_end {
                    rd.dialog_end = false;
                } else {
                    add(&mut out, &name, &p);
                }
                i -= 1;
                need = false;
            }
        } else if d[i] == NAME {
            name = rd.name(&mut i)?;
            if !name.trim().is_empty() {
                if rd.dialog && rd.dialog_begin {
                    add(&mut out, &name, &cur.0);
                } else {
                    need = true;
                }
            }
        }
        i += 1;
    }
    (!out.is_empty()).then_some(out)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let vars = read(d).ok_or("not a Gothic save data file")?;
    // Both games keep the chapter in KAPITEL; other ZenGin files don't have it.
    if !vars.iter().any(|v| v.name == "KAPITEL") {
        return Err("not a Gothic save data file".into());
    }
    let mut b = Builder::new(d);
    let mut seen: HashMap<String, usize> = HashMap::new();
    for v in &vars {
        let full = v.index.map_or_else(|| v.name.clone(), |i| format!("{}[{i}]", v.name));
        let n = seen.entry(full.clone()).or_insert(0);
        *n += 1;
        let label = if *n > 1 { format!("{full} #{n}") } else { full };
        let dialogue = v.name.starts_with("DIA_") || v.name.starts_with("INFO_");
        if dialogue && v.index.is_some() {
            continue;
        }
        let group = if dialogue { "Dialogue flags" } else { "Script variables" };
        b.ranged(&[group, &label], v.at, Kind::I32, i64::from(i32::MIN), i64::from(i32::MAX));
    }
    if b.is_empty() {
        return Err("no variables".into());
    }
    Ok(b.finish(KIND, Checksum::None).with_note(crate::i18n::tr("The script variables of the save (chapter, quest and story flags) and the dialogue flags. Attributes and items are in WORLD.SAV, which is not covered.")))
}
