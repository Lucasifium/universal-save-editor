//! Mass Effect 2 and 3 saves (`.pcsav`), original and Legendary Edition: an Unreal
//! serialised record (version 29 = ME2, 30 = ME2 Legendary, 59 = ME3 and ME3 Legendary)
//! ending in a CRC-32/BZIP2 of everything before it.
//!
//! The order and types of the fields follow Gibbed's Mass Effect 2 and 3 save editors
//! (Rick Gibbed, zlib licence). We walk the file up to the end of the squad records and
//! edit numbers in place; everything else is kept byte for byte.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

const ME2_DIFFICULTY: [&str; 5] = ["Casual", "Normal", "Veteran", "Hardcore", "Insanity"];
const ME3_DIFFICULTY: [&str; 6] = ["Narrative", "Casual", "Normal", "Hardcore", "Insanity", "What Is Beyond Insanity"];

pub fn looks(d: &[u8]) -> bool {
    if d.len() < 64 {
        return false;
    }
    let v = u32::from_le_bytes(d[..4].try_into().unwrap());
    if !matches!(v, 29 | 30 | 59) {
        return false;
    }
    let n = d.len() - 4;
    super::crc32_bzip2(&d[..n]).to_le_bytes() == d[n..]
}

struct R<'a, 'b> {
    b: &'b mut Builder<'a>,
    pos: usize,
    version: u32,
}

impl R<'_, '_> {
    fn i32(&mut self) -> Result<i32, String> {
        let v = self.b.d.get(self.pos..self.pos + 4).ok_or("the save ends early")?;
        self.pos += 4;
        Ok(i32::from_le_bytes(v.try_into().unwrap()))
    }
    fn skip(&mut self, n: usize) -> Result<(), String> {
        if self.pos + n > self.b.d.len() {
            return Err("the save ends early".into());
        }
        self.pos += n;
        Ok(())
    }
    fn count(&mut self) -> Result<usize, String> {
        let n = self.i32()?;
        if !(0..0x7F_FFFF).contains(&n) {
            return Err(format!("bad list length {n} at {}", self.pos - 4));
        }
        Ok(n as usize)
    }
    fn string(&mut self) -> Result<String, String> {
        let n = self.i32()?;
        if n == 0 {
            return Ok(String::new());
        }
        if n.unsigned_abs() > 1 << 20 {
            return Err(format!("bad string length at {}", self.pos - 4));
        }
        let d = self.b.d;
        if n > 0 {
            let s = d.get(self.pos..self.pos + n as usize).ok_or("the save ends early")?;
            self.pos += n as usize;
            Ok(s.iter().take_while(|&&c| c != 0).map(|&c| super::bethesda_cp1252(c)).collect())
        } else {
            let len = 2 * n.unsigned_abs() as usize;
            let s = d.get(self.pos..self.pos + len).ok_or("the save ends early")?;
            self.pos += len;
            let units: Vec<u16> = s.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&u| u != 0).collect();
            Ok(String::from_utf16_lossy(&units))
        }
    }
    fn strings(&mut self) -> Result<(), String> {
        for _ in 0..self.count()? {
            self.string()?;
        }
        Ok(())
    }
    /// A list of fixed-size records.
    fn fixed_list(&mut self, size: usize) -> Result<(), String> {
        let n = self.count()?;
        self.skip(n * size)
    }
    fn ranged(&mut self, path: &[&str], kind: Kind, lo: i64, hi: i64) -> Result<(), String> {
        self.b.ranged(path, self.pos, kind, lo, hi);
        self.skip(kind.size())
    }
    fn me3(&self) -> bool {
        self.version >= 59
    }

    /// Powers: name, rank, (ME3: evolution choices), class, wheel index.
    fn powers(&mut self, base: &[&str]) -> Result<(), String> {
        for _ in 0..self.count()? {
            let name = self.string()?;
            let rank_at = self.pos;
            self.skip(4)?;
            if self.me3() {
                self.skip(6 * 4)?;
            }
            let class = self.string()?;
            self.skip(4)?;
            let label = if name.is_empty() || name == "None" { class } else { name };
            let mut p: Vec<&str> = base.to_vec();
            p.push(&label);
            self.b.ranged(&p, rank_at, Kind::F32, 0, 6);
        }
        Ok(())
    }

    fn loadout(&mut self) -> Result<(), String> {
        for _ in 0..6 {
            self.string()?;
        }
        Ok(())
    }

    fn weapons(&mut self, base: &[&str]) -> Result<(), String> {
        for i in 0..self.count()? {
            let class = self.string()?;
            self.skip(4)?; // ammo used
            let label = format!("{} ({})", class.rsplit('.').next().unwrap_or(&class).trim_start_matches("SFXWeapon_"), i + 1);
            let mut p: Vec<&str> = base.to_vec();
            p.push(&label);
            p.push("Spare ammo");
            self.b.ranged(&p, self.pos, Kind::I32, 0, 9999);
            self.skip(4 + 4 + 4)?; // total ammo, current, last
            if self.version >= 17 {
                self.string()?;
            }
            if self.me3() {
                self.string()?;
            }
        }
        Ok(())
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Mass Effect 2/3 save".into());
    }
    let version = u32::from_le_bytes(d[..4].try_into().unwrap());
    let mut b = Builder::new(d);
    let game = match version {
        29 => "Mass Effect 2",
        30 => "Mass Effect 2 Legendary Edition",
        _ => "Mass Effect 3",
    };
    b.info(&["Save", "Game"], game);
    let mut r = R { b: &mut b, pos: 4, version };
    let res = walk(&mut r);
    let kind = format!("{game} save");
    let note = match res {
        Ok(()) => "Shepard, resources and the squad are shown; the save's checksum is updated when you save.".to_string(),
        Err(e) => format!("Only the start of this save could be read ({e}); what's shown can still be edited."),
    };
    Ok(b.finish(&kind, Checksum::TrailingBzip2).with_note(note))
}

fn walk(r: &mut R) -> Result<(), String> {
    let me3 = r.me3();
    r.string()?; // debug name
    r.ranged(&["Save", "Seconds played"], Kind::F32, 0, i32::MAX as i64)?;
    r.skip(4)?; // disc
    let level = r.string()?;
    r.b.info(&["Save", "Location"], level);
    if me3 && r.version >= 36 {
        r.string()?;
    }
    let names: &[&str] = if me3 { &ME3_DIFFICULTY } else { &ME2_DIFFICULTY };
    r.b.named(&["Save", "Difficulty"], r.pos, Kind::U8, names);
    r.skip(1)?;
    if me3 && (43..=46).contains(&r.version) {
        r.skip(4)?;
    }
    r.skip(4)?; // end game state
    r.skip(16)?; // time stamp
    r.skip(12 + 12 + 4)?; // location, rotation, loading tip
    // Levels (name, 2 bools), streaming (name, bool), kismet (guid, bool), doors (guid, 2 bytes).
    for _ in 0..r.count()? {
        r.string()?;
        r.skip(8)?;
    }
    for _ in 0..r.count()? {
        r.string()?;
        r.skip(4)?;
    }
    r.fixed_list(20)?;
    r.fixed_list(18)?;
    if me3 && r.version >= 46 {
        r.fixed_list(18)?; // placeables
    }
    r.fixed_list(16)?; // pawns
    player(r)?;
    if r.version == 30 {
        r.skip(4 * 7)?; // Mass Effect 1 import rewards (Legendary Edition)
    }
    // Squad.
    for _ in 0..r.count()? {
        let tag = r.string()?;
        let name = squad_name(&tag);
        r.powers(&["Squad", &name, "Powers"])?;
        let cap = if me3 { 60 } else { 30 };
        r.ranged(&["Squad", &name, "Level"], Kind::I32, 1, cap)?;
        r.ranged(&["Squad", &name, "Talent points"], Kind::I32, 0, 9999)?;
        if r.version >= 23 {
            r.loadout()?;
        }
        if r.version >= 29 {
            r.string()?;
        }
        if me3 {
            if r.version >= 45 {
                for _ in 0..r.count()? {
                    r.string()?;
                    r.strings()?;
                }
            }
            r.ranged(&["Squad", &name, "Grenades"], Kind::I32, 0, 99)?;
            r.weapons(&["Squad", &name, "Weapons"])?;
        }
    }
    plot(r)?;
    Ok(())
}

/// Story flags and numbers the editor has a name for: (is a flag, id, section, label).
/// From Trilogy Save Editor's plot databases (Karlitos, CeCILL-2.1, which allows use under the GPL).
fn plot_names(me3: bool) -> impl Iterator<Item = (bool, usize, &'static str, &'static str)> {
    let text = if me3 { include_str!("data/me3_plot.tsv") } else { include_str!("data/me2_plot.tsv") };
    text.lines().filter_map(|l| {
        let mut c = l.split('\t');
        let (kind, id, section, label) = (c.next()?, c.next()?.parse().ok()?, c.next()?, c.next()?);
        Some((kind == "b", id, section, label))
    })
}

/// The plot table that follows the squad: flags packed 32 to a word, then numbers (a plain
/// list in Mass Effect 2, id and value pairs in Mass Effect 3). Values are changed where they
/// are; a Mass Effect 3 number the save doesn't hold yet can't be added.
fn plot(r: &mut R) -> Result<(), String> {
    let me3 = r.me3();
    let words = r.count()?;
    let flags_at = r.pos;
    r.skip(words * 4)?;
    let n = r.count()?;
    let ints_at = r.pos;
    let mut at: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    if me3 {
        r.skip(n * 8)?;
        for i in 0..n {
            let id = i32::from_le_bytes(r.b.d[ints_at + i * 8..ints_at + i * 8 + 4].try_into().unwrap());
            if id >= 0 {
                at.insert(id as usize, ints_at + i * 8 + 4);
            }
        }
    } else {
        r.skip(n * 4)?;
    }
    let int_at = |id: usize| if me3 { at.get(&id).copied() } else { (id < n).then_some(ints_at + id * 4) };
    let top: &[(usize, &str, i64)] =
        if me3 { &[(10159, "Paragon", 100_000), (10160, "Renegade", 100_000), (10297, "Reputation rank", 100_000), (10380, "Reputation points", 100_000)] } else { &[(2, "Paragon", 100_000), (3, "Renegade", 100_000)] };
    for (id, label, hi) in top {
        if let Some(a) = int_at(*id) {
            r.b.ranged(&["Shepard", label], a, Kind::I32, 0, *hi);
        }
    }
    let mut seen = std::collections::HashSet::new();
    for (flag, id, section, label) in plot_names(me3) {
        if top.iter().any(|t| !flag && t.0 == id) {
            continue;
        }
        // Two entries can share a label; the id keeps them apart.
        let key = if seen.insert((flag, section, label)) { label.to_string() } else { format!("{label} ({id})") };
        if flag {
            if id / 32 < words {
                r.b.bits(&["Story flags", section, &key], flags_at + id / 32 * 4, Kind::U32, (id % 32) as u32, 1, 0, 0, 1);
            }
        } else if let Some(a) = int_at(id) {
            r.b.ranged(&["Story numbers", section, &key], a, Kind::I32, i32::MIN as i64, i32::MAX as i64);
        }
    }
    Ok(())
}

fn player(r: &mut R) -> Result<(), String> {
    let me3 = r.me3();
    let female = r.i32()? != 0;
    r.b.info(&["Shepard", "Female"], female);
    let class = r.string()?;
    r.b.info(&["Shepard", "Class"], class.rsplit('.').next().unwrap_or(&class).trim_start_matches("SFXPawn_Player").to_string());
    if me3 {
        if r.version >= 37 {
            r.skip(4)?;
        }
        if r.version >= 48 {
            r.skip(8)?;
        }
    }
    let cap = if me3 { 60 } else { 30 };
    r.ranged(&["Shepard", "Level"], Kind::I32, 1, cap)?;
    r.ranged(&["Shepard", "Experience"], Kind::F32, 0, i32::MAX as i64)?;
    let first = r.string()?;
    r.b.info(&["Shepard", "First name"], first);
    r.skip(4)?; // last name (string id)
    r.b.named(&["Shepard", "Origin"], r.pos, Kind::U8, &["None", "Spacer", "Colonist", "Earthborn"]);
    r.skip(1)?;
    r.b.named(&["Shepard", "Reputation"], r.pos, Kind::U8, &["None", "Survivor", "War Hero", "Ruthless"]);
    r.skip(1)?;
    r.ranged(&["Shepard", "Talent points"], Kind::I32, 0, 9999)?;
    for _ in 0..3 {
        r.string()?;
    }
    appearance(r)?;
    r.powers(&["Powers"])?;
    if me3 && r.version >= 38 {
        r.fixed_list(8)?; // war assets
    }
    r.weapons(&["Weapons"])?;
    if me3 && r.version >= 32 {
        for _ in 0..r.count()? {
            r.string()?;
            r.strings()?;
        }
    }
    if r.version >= 18 {
        r.loadout()?;
    }
    if me3 && r.version >= 41 {
        r.string()?;
        r.string()?;
    }
    if me3 && r.version >= 33 {
        r.fixed_list(4)?;
    }
    if r.version >= 19 {
        for _ in 0..r.count()? {
            r.string()?;
            if r.version >= 30 && me3 {
                r.string()?;
            } else {
                r.skip(4)?;
            }
        }
    }
    if me3 && r.version >= 44 {
        r.skip(4)?; // current health
    }
    r.ranged(&["Resources", "Credits"], Kind::I32, 0, i32::MAX as i64)?;
    r.ranged(&["Resources", "Medi-gel"], Kind::I32, 0, 999)?;
    r.ranged(&["Resources", "Element zero"], Kind::I32, 0, i32::MAX as i64)?;
    r.ranged(&["Resources", "Iridium"], Kind::I32, 0, i32::MAX as i64)?;
    r.ranged(&["Resources", "Palladium"], Kind::I32, 0, i32::MAX as i64)?;
    r.ranged(&["Resources", "Platinum"], Kind::I32, 0, i32::MAX as i64)?;
    r.ranged(&["Resources", "Probes"], Kind::I32, 0, 9999)?;
    r.ranged(&["Resources", "Fuel"], Kind::F32, 0, 100_000)?;
    if me3 && r.version >= 54 {
        r.ranged(&["Resources", "Grenades"], Kind::I32, 0, 99)?;
    }
    if r.version >= 25 {
        let face = r.string()?;
        r.b.info(&["Shepard", "Face code"], face);
    }
    if r.version >= 26 {
        r.skip(4)?;
    }
    if me3 && r.version >= 42 {
        r.skip(16)?;
    }
    Ok(())
}

fn appearance(r: &mut R) -> Result<(), String> {
    r.skip(1 + 13 * 4)?;
    let morph = r.i32()? != 0;
    if morph {
        r.string()?; // hair
        r.strings()?; // accessories
        for _ in 0..r.count()? {
            r.string()?;
            r.skip(4)?;
        }
        for _ in 0..r.count()? {
            r.string()?;
            r.skip(12)?;
        }
        for _ in 0..4 {
            r.fixed_list(12)?; // vertices
        }
        for _ in 0..r.count()? {
            r.string()?;
            r.skip(4)?;
        }
        for _ in 0..r.count()? {
            r.string()?;
            r.skip(16)?;
        }
        for _ in 0..r.count()? {
            r.string()?;
            r.string()?;
        }
    }
    if r.me3() && r.version >= 55 {
        r.skip(4)?;
    }
    Ok(())
}

/// "hench_vixen" → "Miranda".
fn squad_name(tag: &str) -> String {
    let t = tag.to_lowercase();
    let t = t.trim_start_matches("hench_");
    let n = match t {
        "vixen" => "Miranda",
        "leading" => "Jacob",
        "convict" => "Jack",
        "garrus" => "Garrus",
        "geth" => "Legion",
        "grunt" => "Grunt",
        "mystic" => "Samara",
        "professor" => "Mordin",
        "tali" => "Tali",
        "thief" => "Kasumi",
        "veteran" => "Zaeed",
        "assassin" => "Thane",
        "liara" => "Liara",
        "kaidan" => "Kaidan",
        "ashley" => "Ashley",
        "marine" => "James",
        "prothean" => "Javik",
        "edi" => "EDI",
        "anderson" => "Anderson",
        "wrex" => "Wrex",
        "aria" => "Aria",
        "nyreen" => "Nyreen",
        _ => {
            let mut c = t.chars();
            return c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_else(|| tag.to_string());
        }
    };
    n.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    /// Research aid: SAVE_EDITOR_ME = a .pcsav to read.
    #[test]
    #[ignore]
    fn real_save() {
        let path = std::env::var("SAVE_EDITOR_ME").expect("SAVE_EDITOR_ME");
        let d = std::fs::read(path).unwrap();
        let doc = parse(&d).unwrap();
        println!("{}\n{}", doc.note.clone().unwrap_or_default(), serde_json::to_string_pretty(&doc.view).unwrap());
        assert_eq!(doc.to_bytes().unwrap(), d);
    }

    /// Research aid: SAVE_EDITOR_ME = a Mass Effect 2 or 3 .pcsav. Changes Paragon and flips
    /// one story flag, then checks only those changed when the save is read again.
    #[test]
    #[ignore]
    fn real_save_story_edit() {
        let path = std::env::var("SAVE_EDITOR_ME").expect("SAVE_EDITOR_ME");
        let d = std::fs::read(path).unwrap();
        let mut doc = parse(&d).unwrap();
        let before = doc.view.clone();
        let (sec, flag) = {
            let flags = before["Story flags"].as_object().unwrap();
            let (sec, m) = flags.iter().next().unwrap();
            (sec.clone(), m.as_object().unwrap().keys().next().unwrap().clone())
        };
        let fp = format!("/Story flags/{}/{}", crate::tree::esc(&sec), crate::tree::esc(&flag));
        let was = before.pointer(&fp).unwrap().as_i64().unwrap();
        doc.set(&fp, Value::from(1 - was)).unwrap();
        doc.set("/Shepard/Paragon", Value::from(777)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let again = parse(&out).unwrap().view;
        assert_eq!(again.pointer(&fp), Some(&Value::from(1 - was)));
        assert_eq!(again.pointer("/Shepard/Paragon"), Some(&Value::from(777)));
        let mut expect = before;
        *expect.pointer_mut(&fp).unwrap() = Value::from(1 - was);
        *expect.pointer_mut("/Shepard/Paragon").unwrap() = Value::from(777);
        assert_eq!(again, expect);
        // Outside the trailing checksum, one flag bit and one number changed.
        let diff = d.iter().zip(&out).take(d.len() - 4).filter(|(a, b)| a != b).count();
        assert!((2..=5).contains(&diff), "{diff} bytes changed");
    }
}

#[cfg(test)]
mod sweep {
    /// Research aid: reads every .pcsav under SAVE_EDITOR_ME_DIR (read-only) and reports
    /// any the walker can't follow to the end.
    #[test]
    #[ignore]
    fn all_saves() {
        fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("pcsav")) {
                    out.push(p);
                }
            }
        }
        let mut files = vec![];
        walk(std::path::Path::new(&std::env::var("SAVE_EDITOR_ME_DIR").unwrap()), &mut files);
        let (mut ok, mut partial, mut other) = (0, 0, 0);
        for f in &files {
            let d = std::fs::read(f).unwrap();
            if !super::looks(&d) {
                other += 1;
                continue;
            }
            let doc = super::parse(&d).unwrap();
            if doc.note.as_deref().is_some_and(|n| n.starts_with("Only")) {
                partial += 1;
                println!("{}: {}", f.display(), doc.note.unwrap());
            } else {
                ok += 1;
            }
        }
        println!("{ok} read fully, {partial} partly, {other} not ME2/ME3 saves");
    }
}
