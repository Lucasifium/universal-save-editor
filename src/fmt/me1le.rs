//! Mass Effect 1 (Legendary Edition) game data, once the save's zlib blocks are
//! unpacked: an Unreal serialised record starting with version 50. Its order follows
//! Trilogy Save Editor (Karlitos, CeCILL 2.1): plot table, journal, codex, time stamps,
//! then Shepard (class, level, talents, inventory, credits, medi-gel…) and the squad.
//!
//! Bools are 4 bytes, enums 1 byte, lists a count then their items, text an FString.
//! Numbers are changed in place; nothing moves.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::collections::HashMap;
use std::sync::OnceLock;

const CLASSES: [&str; 6] = ["Soldier", "Engineer", "Adept", "Infiltrator", "Sentinel", "Vanguard"];
const DIFFICULTY: [&str; 5] = ["Casual", "Normal", "Veteran", "Hardcore", "Insanity"];
/// Talent names by id, from Trilogy Save Editor's bonus talent list; others show their number.
const TALENTS: [(i32, &str); 15] = [
    (7, "Assault Rifles"),
    (15, "Shotguns"),
    (21, "Sniper Rifles"),
    (49, "Throw"),
    (50, "Lift"),
    (56, "Warp"),
    (57, "Singularity"),
    (63, "Barrier"),
    (64, "Stasis"),
    (84, "Electronics"),
    (86, "Damping"),
    (91, "Hacking"),
    (93, "Decryption"),
    (98, "First Aid"),
    (99, "Medicine"),
];
/// A talent's name, or "Talent <id>" for ones without a known name.
pub fn talent_name(id: i32) -> String {
    TALENTS.iter().find(|t| t.0 == id).map_or_else(|| format!("Talent {id}"), |t| t.1.to_string())
}

const LEVELS: [&str; 11] = ["-", "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X"];

/// (item id, manufacturer id) → name, from Trilogy Save Editor's item list.
fn item_names() -> &'static HashMap<(i32, i32), String> {
    static T: OnceLock<HashMap<(i32, i32), String>> = OnceLock::new();
    T.get_or_init(|| {
        include_str!("data/me1_items.tsv")
            .lines()
            .filter_map(|l| {
                let mut f = l.splitn(3, '\t');
                Some(((f.next()?.parse().ok()?, f.next()?.parse().ok()?), f.next()?.to_string()))
            })
            .collect()
    })
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 1024 && d[..4] == 50i32.to_le_bytes() && {
        // Character id: a short text right after the version.
        let n = i32::from_le_bytes(d[4..8].try_into().unwrap());
        (-200..200).contains(&n) && n != 0
    }
}

struct R<'a, 'b> {
    b: &'b mut Builder<'a>,
    pos: usize,
}

impl R<'_, '_> {
    fn i32(&mut self) -> Result<i32, String> {
        let v = self.b.d.get(self.pos..self.pos + 4).ok_or("the save ends early")?;
        self.pos += 4;
        Ok(i32::from_le_bytes(v.try_into().unwrap()))
    }
    fn u8(&mut self) -> Result<u8, String> {
        let v = *self.b.d.get(self.pos).ok_or("the save ends early")?;
        self.pos += 1;
        Ok(v)
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
    fn bool(&mut self) -> Result<bool, String> {
        match self.i32()? {
            0 => Ok(false),
            1 => Ok(true),
            v => Err(format!("expected 0 or 1, found {v} at {}", self.pos - 4)),
        }
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
    fn list_of(&mut self, size: usize) -> Result<usize, String> {
        let n = self.count()?;
        self.skip(n * size)?;
        Ok(n)
    }
    fn ranged(&mut self, path: &[&str], kind: Kind, lo: i64, hi: i64) -> Result<(), String> {
        self.b.ranged(path, self.pos, kind, lo, hi);
        self.skip(kind.size())
    }
    fn float(&mut self, path: &[&str]) -> Result<(), String> {
        self.b.field(path, self.pos, Kind::F32);
        self.skip(4)
    }

    /// Simple talents (id, rank) and complex ones (id, rank, max, offset, per rank,
    /// order, prerequisites). Ranks become editable under `base`.
    fn talents(&mut self, base: &[&str]) -> Result<(), String> {
        for _ in 0..self.count()? {
            self.skip(8)?;
        }
        for _ in 0..self.count()? {
            let id = self.i32()?;
            let rank_at = self.pos;
            self.skip(4)?;
            let max = self.i32()?;
            self.skip(12)?;
            self.list_of(4)?;
            self.list_of(4)?;
            let label = talent_name(id);
            let mut p: Vec<&str> = base.to_vec();
            p.push(&label);
            self.b.ranged(&p, rank_at, Kind::I32, 0, max.clamp(1, 20) as i64);
        }
        Ok(())
    }

    /// A list of items: id, level (1 byte), manufacturer, plot id, new, junk, mods.
    fn items(&mut self, base: &[&str], names: &mut HashMap<String, usize>) -> Result<(), String> {
        for _ in 0..self.count()? {
            let id = self.i32()?;
            let level_at = self.pos;
            self.skip(1)?;
            let maker = self.i32()?;
            self.skip(4)?;
            self.bool()?;
            self.bool()?;
            for _ in 0..self.count()? {
                self.skip(4 + 1 + 4 + 4)?;
            }
            if id == 0 && maker == 0 {
                continue; // an empty slot
            }
            let name = item_names().get(&(id, maker)).cloned().unwrap_or_else(|| format!("Item #{id}"));
            let n = names.entry(name.clone()).or_default();
            *n += 1;
            let label = if *n > 1 { format!("{name} ({n})") } else { name };
            let mut p: Vec<&str> = base.to_vec();
            p.push(&label);
            self.b.named(&p, level_at, Kind::U8, &LEVELS);
        }
        Ok(())
    }
}

fn squad_name(tag: &str) -> String {
    let t = tag.to_lowercase();
    let t = t.trim_start_matches("hench_");
    match t {
        "humanfemale" => "Ashley".into(),
        "humanmale" => "Kaidan".into(),
        "asari" => "Liara".into(),
        "turian" => "Garrus".into(),
        "krogan" => "Wrex".into(),
        "quarian" => "Tali".into(),
        _ => tag.to_string(),
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not Mass Effect 1 Legendary Edition data".into());
    }
    let mut b = Builder::new(d);
    b.info(&["Save", "Game"], "Mass Effect Legendary Edition (ME1)");
    let mut r = R { b: &mut b, pos: 4 };
    let res = walk(&mut r);
    let note = match res {
        Ok(()) => "Shepard, resources, inventory and the squad are shown; the save is packed again and its checksum updated when you save.".to_string(),
        Err(e) => format!("Only the start of this save could be read ({e}); what's shown can still be edited."),
    };
    Ok(b.finish("Mass Effect 1 Legendary Edition save", Checksum::None).with_note(note))
}

fn walk(r: &mut R) -> Result<(), String> {
    r.string()?; // character id
    r.skip(16)?; // created
    // Plot table: booleans (bit fields), integers, floats.
    r.list_of(4)?;
    let ints_at = r.pos + 4;
    let ints = r.list_of(4)?;
    // Paragon and Renegade are plot integers 47 and 46.
    for (i, label) in [(47, "Paragon"), (46, "Renegade")] {
        if i < ints {
            r.b.ranged(&["Shepard", label], ints_at + 4 * i, Kind::I32, 0, 9999);
        }
    }
    r.list_of(4)?;
    // Journal.
    r.skip(4)?;
    for _ in 0..r.count()? {
        r.skip(4)?;
        r.bool()?;
        r.list_of(4)?;
    }
    r.list_of(4)?;
    // Codex.
    for _ in 0..r.count()? {
        r.list_of(8)?;
    }
    r.list_of(4)?;
    r.skip(16)?; // time stamp
    r.ranged(&["Save", "Seconds played"], Kind::I32, 0, i32::MAX as i64)?;
    player(r)?;
    r.string()?; // base level
    let map = r.string()?;
    r.b.info(&["Save", "Location"], map);
    r.string()?;
    r.skip(24)?; // location, rotation
    let mut names = HashMap::new();
    for _ in 0..r.count()? {
        let tag = r.string()?;
        let name = squad_name(&tag);
        r.talents(&["Squad", &name, "Talents"])?;
        r.items(&["Squad", &name, "Equipment"], &mut names)?;
        r.items(&["Squad", &name, "Quick slots"], &mut names)?;
        r.ranged(&["Squad", &name, "Talent points"], Kind::I32, 0, 9999)?;
        r.skip(4 * 4 + 1 + 8 + 2 + 4 + 16 + 2 + 8)?; // pool, auto level-up, names … health, shield
        r.ranged(&["Squad", &name, "Level"], Kind::I32, 1, 60)?;
        r.bool()?;
        r.skip(1 + 4)?;
    }
    Ok(())
}

fn player(r: &mut R) -> Result<(), String> {
    let female = r.bool()?;
    r.b.info(&["Shepard", "Female"], female);
    r.skip(4)?; // class name (string id)
    let class = r.u8()? as usize;
    r.b.info(&["Shepard", "Class"], CLASSES.get(class).copied().unwrap_or("?"));
    r.ranged(&["Shepard", "Level"], Kind::I32, 1, 60)?;
    r.float(&["Shepard", "Experience"])?;
    let first = r.string()?;
    r.b.info(&["Shepard", "First name"], first);
    r.skip(4)?; // last name (string id)
    r.b.named(&["Shepard", "Origin"], r.pos, Kind::U8, &["None", "Spacer", "Colonist", "Earthborn"]);
    r.skip(1)?;
    r.b.named(&["Shepard", "Reputation"], r.pos, Kind::U8, &["None", "Survivor", "War Hero", "Ruthless"]);
    r.skip(1)?;
    r.skip(4 + 1)?; // specialization, spectre rank
    r.ranged(&["Shepard", "Talent points"], Kind::I32, 0, 9999)?;
    r.skip(4)?; // talent pool points
    r.string()?; // mapped talent
    if r.bool()? {
        head_morph(r)?;
    }
    r.talents(&["Powers"])?;
    let mut names = HashMap::new();
    for part in ["Equipment", "Quick slots", "Inventory", "Buy pack"] {
        r.items(&["Items", part], &mut names)?;
    }
    r.ranged(&["Resources", "Credits"], Kind::I32, 0, 9_999_999)?;
    r.ranged(&["Resources", "Medi-gel"], Kind::I32, 0, 999)?;
    r.float(&["Resources", "Grenades"])?;
    r.float(&["Resources", "Omni-gel"])?;
    r.string()?; // face code
    r.bool()?;
    r.skip(4 + 4 + 4 + 1 + 4 + 16 + 2 + 12)?; // auto level-up … skills
    r.skip(4 + 4 + 4)?; // health, shield, xp level
    r.bool()?;
    let opts_at = r.pos + 4;
    if r.list_of(4)? > 0 {
        r.b.named(&["Save", "Difficulty"], opts_at, Kind::I32, &DIFFICULTY);
    }
    r.bool()?; // helmet shown
    r.skip(5)?;
    r.string()?; // last power
    r.skip(4)?; // max health
    r.list_of(8)?; // hotkeys
    r.string()?; // primary weapon
    r.string()?; // secondary weapon
    Ok(())
}

fn head_morph(r: &mut R) -> Result<(), String> {
    r.string()?;
    for _ in 0..r.count()? {
        r.string()?;
    }
    for _ in 0..r.count()? {
        r.string()?;
        r.skip(4)?;
    }
    for _ in 0..r.count()? {
        r.string()?;
        r.skip(12)?;
    }
    for _ in 0..4 {
        r.list_of(12)?;
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
    Ok(())
}

#[cfg(test)]
mod tests {
    fn i(o: &mut Vec<u8>, v: i32) {
        o.extend(v.to_le_bytes());
    }
    fn f(o: &mut Vec<u8>, v: f32) {
        o.extend(v.to_le_bytes());
    }
    fn s(o: &mut Vec<u8>, t: &str) {
        i(o, t.len() as i32 + 1);
        o.extend(t.as_bytes());
        o.push(0);
    }
    fn item(o: &mut Vec<u8>, id: i32, level: u8, maker: i32) {
        i(o, id);
        o.push(level);
        i(o, maker);
        i(o, 0);
        i(o, 0);
        i(o, 0);
        i(o, 0); // no mods
    }

    /// A synthetic ME1 LE record: Shepard, one item, one squad member.
    fn sample() -> Vec<u8> {
        let mut d = vec![];
        i(&mut d, 50);
        s(&mut d, "Test");
        d.extend([0u8; 16]);
        i(&mut d, 0); // plot booleans
        i(&mut d, 48);
        for n in 0..48 {
            i(&mut d, if n == 47 { 120 } else if n == 46 { 30 } else { 0 });
        }
        i(&mut d, 0); // floats
        i(&mut d, 0); // journal counter
        i(&mut d, 0);
        i(&mut d, 0);
        i(&mut d, 0); // codex
        i(&mut d, 0);
        d.extend([0u8; 16]);
        i(&mut d, 3600);
        // Shepard
        i(&mut d, 1);
        i(&mut d, 0);
        d.push(2); // Adept
        i(&mut d, 20);
        f(&mut d, 5000.0);
        s(&mut d, "Jane");
        i(&mut d, 0);
        d.push(1);
        d.push(2);
        i(&mut d, 0);
        d.push(0);
        i(&mut d, 3);
        i(&mut d, 0);
        s(&mut d, "");
        i(&mut d, 0); // no head morph
        i(&mut d, 0); // simple talents
        i(&mut d, 1); // one complex talent: Lift
        for v in [50, 4, 12, 0, 1, 0] {
            i(&mut d, v);
        }
        i(&mut d, 0);
        i(&mut d, 0);
        i(&mut d, 1);
        item(&mut d, 288, 7, 451);
        for _ in 0..3 {
            i(&mut d, 0);
        }
        i(&mut d, 12345); // credits
        i(&mut d, 5);
        f(&mut d, 4.0);
        f(&mut d, 100.0);
        s(&mut d, "");
        i(&mut d, 0);
        d.extend([0u8; 47]);
        d.extend([0u8; 12]);
        i(&mut d, 0);
        i(&mut d, 2); // game options: difficulty first
        i(&mut d, 3);
        i(&mut d, 0);
        i(&mut d, 0);
        d.extend([0u8; 5]);
        s(&mut d, "");
        f(&mut d, 100.0);
        i(&mut d, 0);
        s(&mut d, "");
        s(&mut d, "");
        s(&mut d, "");
        s(&mut d, "MapTest");
        s(&mut d, "");
        d.extend([0u8; 24]);
        // Squad
        i(&mut d, 1);
        s(&mut d, "hench_asari");
        i(&mut d, 0);
        i(&mut d, 0);
        i(&mut d, 0);
        i(&mut d, 0);
        i(&mut d, 6); // talent points
        d.extend([0u8; 16 + 1 + 8 + 2 + 4 + 16 + 2 + 8]);
        i(&mut d, 21); // level
        i(&mut d, 0);
        d.extend([0u8; 5]);
        d.extend([0u8; 1024]);
        d
    }

    #[test]
    fn reads_and_edits() {
        let d = sample();
        let mut doc = super::parse(&d).unwrap();
        assert!(doc.note.as_deref().unwrap().starts_with("Shepard"), "{:?}", doc.note);
        let v = &doc.view;
        assert_eq!(v.pointer("/Shepard/Class"), Some(&"Adept".into()));
        assert_eq!(v.pointer("/Shepard/Paragon"), Some(&120.into()));
        assert_eq!(v.pointer("/Shepard/Origin"), Some(&"Spacer".into()));
        assert_eq!(v.pointer("/Save/Difficulty"), Some(&"Hardcore".into()));
        assert_eq!(v.pointer("/Save/Location"), Some(&"MapTest".into()));
        assert_eq!(v.pointer("/Powers/Lift"), Some(&4.into()));
        assert_eq!(v.pointer("/Items/Equipment/[Aldrin Labs] Agent (Turian Light Armor)"), Some(&"VII".into()));
        assert_eq!(v.pointer("/Squad/Liara/Level"), Some(&21.into()));
        doc.set("/Resources/Credits", 999_999.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        assert_eq!(super::parse(&out).unwrap().view.pointer("/Resources/Credits"), Some(&999_999.into()));
    }

    /// Research aid: SAVE_EDITOR_ME1LE = unpacked ME1 LE data.
    #[test]
    #[ignore]
    fn real_save() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_ME1LE").unwrap()).unwrap();
        let doc = super::parse(&d).unwrap();
        println!("{}\n{}", doc.note.clone().unwrap_or_default(), serde_json::to_string_pretty(&doc.view).unwrap());
        assert_eq!(doc.to_bytes().unwrap(), d);
    }
}
