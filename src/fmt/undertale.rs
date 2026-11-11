//! Undertale (`file0`, `file9`, `file8`) and Deltarune (`filech1_0` … `filech4_2`) saves.
//!
//! Both games (GameMaker) write one value per line: text as is, numbers with GameMaker's
//! `string()` (up to 6 significant digits, so a big play time reads `1.56599e+06`), each
//! line ending in a space and CRLF, the last line without the CRLF. Nothing is
//! encrypted or checked. A line's value is rewritten on its own, so its trailing space,
//! the line endings and every other line stay exactly as they were.
//!
//! Line maps: Undertale from FloweysTimeMachine's notes (no licence, facts only) checked
//! on sample saves; Deltarune Chapter 1 from the game's own save routine order (facts only),
//! Chapters 2–4 only for the values checked on sample saves (gold, party, stats, equipment).

use crate::tree::{num, Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;

/// Undertale item ids (the game's own item numbering), 0 = empty.
pub const UT_ITEMS: [&str; 65] = [
    "(empty)", "Monster Candy", "Croquet Roll", "Stick", "Bandage", "Rock Candy", "Pumpkin Rings", "Spider Donut", "Stoic Onion",
    "Ghost Fruit", "Spider Cider", "Butterscotch Pie", "Faded Ribbon", "Toy Knife", "Tough Glove", "Manly Bandanna", "Snowman Piece",
    "Nice Cream", "Puppydough Icecream", "Bisicle", "Unisicle", "Cinnamon Bunny", "Temmie Flakes", "Abandoned Quiche", "Old Tutu",
    "Ballet Shoes", "Punch Card", "Annoying Dog", "Dog Salad", "Dog Residue (1)", "Dog Residue (2)", "Dog Residue (3)",
    "Dog Residue (4)", "Dog Residue (5)", "Dog Residue (6)", "Astronaut Food", "Instant Noodles", "Crab Apple", "Hot Dog...?",
    "Hot Cat", "Glamburger", "Sea Tea", "Starfait", "Legendary Hero", "Cloudy Glasses", "Torn Notebook", "Stained Apron",
    "Burnt Pan", "Cowboy Hat", "Empty Gun", "Heart Locket", "Worn Dagger", "Real Knife", "The Locket", "Bad Memory", "Dream",
    "Undyne's Letter", "Undyne Letter EX", "Potato Chisps", "Junk Food", "Mystery Key", "Face Steak", "Hush Puppy", "Snail Pie",
    "Temmie Armor",
];

/// Undertale phone slots: call options (ids above 200), 0 = empty.
const UT_PHONE: [(i64, &str); 10] = [
    (0, "(empty)"),
    (201, "Say Hello"),
    (202, "Puzzle Help"),
    (203, "About Yourself"),
    (204, "Call Her \"Mom\""),
    (205, "Flirt"),
    (206, "Toriel's Phone"),
    (210, "Papyrus's Phone"),
    (220, "Dimensional Box A"),
    (221, "Dimensional Box B"),
];

/// Deltarune party member numbers (Noelle joins in Chapter 2).
const DR_MEMBERS: [(i64, &str); 5] = [(0, "(nobody)"), (1, "Kris"), (2, "Susie"), (3, "Ralsei"), (4, "Noelle")];

#[derive(Clone, Debug)]
enum Kind {
    /// A number (written as a whole number when it is one).
    Num,
    /// Free text (names).
    Text,
    /// A number shown by name from a fixed list; numbers not in the list stay numbers.
    Named(Arc<Vec<(i64, String)>>),
}

#[derive(Clone, Debug)]
struct Leaf {
    /// Byte range of the value (without the trailing space and line ending).
    start: usize,
    end: usize,
    kind: Kind,
}

#[derive(Clone, Debug)]
struct LineBackend {
    raw: Arc<Vec<u8>>,
    leaves: Vec<Leaf>,
}

/// Is this GameMaker number text (`12`, `-38`, `145.75`, `1.80188e+006`)?
fn numeric(s: &str) -> bool {
    !s.is_empty() && s.bytes().any(|b| b.is_ascii_digit()) && s.bytes().all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'-' | b'+' | b'e' | b'E')) && s.parse::<f64>().is_ok()
}

/// A number line as shown: whole numbers as integers (also `1.56599e+06`).
fn num_value(s: &str) -> Value {
    if let Ok(i) = s.parse::<i64>() {
        return Value::from(i);
    }
    let f: f64 = s.parse().unwrap_or(0.0);
    if f.fract() == 0.0 && f.abs() < 9.0e15 {
        Value::from(f as i64)
    } else {
        num(f)
    }
}

/// The text a number is written as.
fn num_text(v: &Value) -> Result<String, String> {
    let f = match v {
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                return Ok(i.to_string());
            }
            n.as_f64().ok_or("not a number")?
        }
        Value::Bool(b) => return Ok(i64::from(*b).to_string()),
        Value::String(s) if numeric(s.trim()) => s.trim().parse::<f64>().map_err(|e| e.to_string())?,
        _ => return Err("must be a number".into()),
    };
    if !f.is_finite() {
        return Err("must be a number".into());
    }
    Ok(if f.fract() == 0.0 && f.abs() < 9.0e15 { (f as i64).to_string() } else { format!("{f}") })
}

fn named_value(list: &[(i64, String)], s: &str) -> Value {
    let n = num_value(s);
    match n.as_i64().and_then(|id| list.iter().find(|e| e.0 == id)) {
        Some(e) => Value::String(e.1.clone()),
        None => n,
    }
}

impl LineBackend {
    fn text(&self, leaf: usize, v: &Value) -> Result<String, String> {
        match &self.leaves[leaf].kind {
            Kind::Num => num_text(v),
            Kind::Text => match v {
                Value::String(s) if s.contains(['\n', '\r']) => Err("can't contain line breaks".into()),
                Value::String(s) => Ok(s.clone()),
                Value::Number(n) => Ok(n.to_string()),
                _ => Err("must be text".into()),
            },
            Kind::Named(list) => match v {
                Value::String(s) => match list.iter().find(|e| e.1 == *s) {
                    Some(e) => Ok(e.0.to_string()),
                    None if numeric(s.trim()) => num_text(v),
                    None => Err(format!("\u{201C}{s}\u{201D} isn't one of the choices")),
                },
                _ => num_text(v),
            },
        }
    }
}

impl Backend for LineBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = crate::tree::Patches::default();
        for (leaf, v) in changes {
            let l = &self.leaves[*leaf];
            p.add(l.start, l.end, self.text(*leaf, v)?.into_bytes());
        }
        p.apply(&self.raw)
    }

    /// Shows exactly what reading the saved line back gives.
    fn normalize(&self, leaf: usize, v: Value) -> Value {
        match (&self.leaves[leaf].kind, self.text(leaf, &v)) {
            (Kind::Num, Ok(t)) => num_value(&t),
            (Kind::Named(list), Ok(t)) => named_value(list, &t),
            _ => v,
        }
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        self.text(leaf, v).map(|_| ())
    }

    fn choices(&self, leaf: usize, _view: &Value) -> Option<Arc<Vec<String>>> {
        match &self.leaves[leaf].kind {
            Kind::Named(list) => Some(Arc::new(list.iter().map(|e| e.1.clone()).collect())),
            _ => None,
        }
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

/// The value of each line, with its byte range.
fn split_lines(text: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut at = 0;
    for line in text.split('\n') {
        let body = line.strip_suffix('\r').unwrap_or(line);
        let value = body.trim_end_matches(' ');
        out.push((at, at + value.len()));
        at += line.len() + 1;
    }
    out
}

/// Builds the view: sections of named lines, then every other line under "Other lines".
struct Doc<'a> {
    text: &'a str,
    lines: Vec<(usize, usize)>,
    used: Vec<bool>,
    root: Map<String, Value>,
    editable: HashMap<String, usize>,
    leaves: Vec<Leaf>,
}

impl<'a> Doc<'a> {
    fn new(text: &'a str) -> Self {
        let lines = split_lines(text);
        let used = vec![false; lines.len()];
        Self { text, lines, used, root: Map::new(), editable: HashMap::new(), leaves: vec![] }
    }

    fn get(&self, line: usize) -> &str {
        let (s, e) = self.lines[line - 1];
        &self.text[s..e]
    }

    /// Put 1-based `line` at `section/key` (a section path may have one `/`).
    fn add(&mut self, section: &str, key: &str, line: usize, kind: Kind) {
        let Some(&(s, e)) = self.lines.get(line - 1) else { return };
        let raw = &self.text[s..e];
        let kind = match kind {
            Kind::Num if !numeric(raw) => Kind::Text,
            Kind::Named(_) if !numeric(raw) => Kind::Text,
            k => k,
        };
        let v = match &kind {
            Kind::Num => num_value(raw),
            Kind::Text => Value::String(raw.to_string()),
            Kind::Named(list) => named_value(list, raw),
        };
        let mut map = &mut self.root;
        let mut ptr = String::new();
        for part in section.split('/') {
            ptr = crate::tree::child(&ptr, part);
            map = map.entry(part.to_string()).or_insert_with(|| Value::Object(Map::new())).as_object_mut().unwrap();
        }
        map.insert(key.to_string(), v);
        self.editable.insert(crate::tree::child(&ptr, key), self.leaves.len());
        self.leaves.push(Leaf { start: s, end: e, kind });
        self.used[line - 1] = true;
    }

    fn finish(mut self, kind_name: &str, note: &str) -> TreeDoc {
        // Everything not shown above, by line number, so every value can still be edited.
        for line in 1..=self.lines.len() {
            if !self.used[line - 1] && !(line == self.lines.len() && self.get(line).is_empty()) {
                let kind = if numeric(self.get(line)) { Kind::Num } else { Kind::Text };
                self.add("Other lines", &format!("Line {line}"), line, kind);
            }
        }
        let backend = LineBackend { raw: Arc::new(self.text.as_bytes().to_vec()), leaves: self.leaves };
        TreeDoc::new(Value::Object(self.root), self.editable, kind_name, Box::new(backend)).with_advanced(vec!["Other lines".into()]).with_note(note)
    }
}

fn named(list: &[(i64, &str)]) -> Kind {
    Kind::Named(Arc::new(list.iter().map(|&(i, n)| (i, n.to_string())).collect()))
}

fn ut_items() -> Kind {
    Kind::Named(Arc::new(UT_ITEMS.iter().enumerate().map(|(i, n)| (i as i64, n.to_string())).collect()))
}

/// Line count of a text in this style (the last line has no line break).
fn line_count(text: &str) -> usize {
    text.bytes().filter(|&b| b == b'\n').count() + 1
}

pub const UNDERTALE: &str = "Undertale save";
pub const DELTARUNE_CH1: &str = "Deltarune save (Chapter 1)";
pub const DELTARUNE_LATER: &str = "Deltarune save (Chapters 2\u{2013}4)";

/// Undertale `file0` / `file9` / `file8`: 549 lines.
pub fn parse_undertale(text: &str) -> Result<TreeDoc, String> {
    if line_count(text) != 549 || !text.contains('\r') {
        return Err("not an Undertale save".into());
    }
    let mut d = Doc::new(text);
    // Stats, inventory, equipment, room and time are all numbers; LV is 1 to 20.
    if !(2..=30).chain([548, 549]).all(|l| numeric(d.get(l))) || !d.get(2).parse::<f64>().is_ok_and(|lv| (1.0..=20.0).contains(&lv)) {
        return Err("not an Undertale save".into());
    }
    let c = "Character";
    d.add(c, "Name", 1, Kind::Text);
    for (k, l) in [("LV", 2), ("Max HP", 3), ("AT", 5), ("Weapon AT", 6), ("DF", 7), ("Armor DF", 8), ("Speed", 9), ("EXP", 10), ("Gold", 11), ("Kills", 12)] {
        d.add(c, k, l, Kind::Num);
    }
    for i in 0..8 {
        d.add("Items", &format!("Slot {}", i + 1), 13 + 2 * i, ut_items());
    }
    for i in 0..8 {
        d.add("Phone", &format!("Slot {}", i + 1), 14 + 2 * i, named(&UT_PHONE));
    }
    d.add("Equipment", "Weapon", 29, ut_items());
    d.add("Equipment", "Armor", 30, ut_items());
    // Route and counter lines (line numbers as in the game's own file).
    for (k, l) in [
        ("Total kills (genocide count)", 232),
        ("Kills: Ruins", 233),
        ("Kills: Snowdin", 234),
        ("Kills: Waterfall", 235),
        ("Kills: Hotland", 236),
        ("Genocide route: Ruins", 252),
        ("Genocide route: Snowdin", 253),
        ("Genocide route: Waterfall", 254),
        ("Genocide route: Hotland", 255),
        ("Genocide route: Core", 256),
        ("True Pacifist done", 38),
        ("Hard mode", 37),
        ("Disable random encounters", 39),
        ("Killed Flowey", 506),
        ("Killed Asgore", 507),
        ("Killed Mettaton", 456),
        ("Killed Undyne the Undying", 282),
        ("fun value", 36),
        ("Monsters spared", 54),
        ("Fled from battle", 55),
        ("Dialogue skipped", 56),
        ("Gold spent: Snowdin shop", 90),
        ("Gold spent: Hotland shop", 91),
        ("Gold spent: Core shop", 409),
        ("Flowey stalking", 40),
    ] {
        if numeric(d.get(l)) {
            d.add("Route and counters", k, l, Kind::Num);
        }
    }
    d.add("Progress", "Room", 548, Kind::Num);
    d.add("Progress", "Play time (frames)", 549, Kind::Num);
    Ok(d.finish(
        UNDERTALE,
        "Undertale save: each value is one line of the file. The load menu shows what undertale.ini (in the same folder) says; \
         the game reads this file when you continue. Current HP isn't saved (the game refills it on load).",
    ))
}

/// Deltarune `filechN_S`: 10318 lines (Chapter 1) or 3055 lines (Chapters 2–4).
pub fn parse_deltarune(text: &str) -> Result<TreeDoc, String> {
    let n = line_count(text);
    let ch1 = match n {
        10318 => true,
        3055 => false,
        _ => return Err("not a Deltarune save".into()),
    };
    let mut d = Doc::new(text);
    if !text.contains('\r') || !(8..=16).chain([n - 2, n - 1, n]).all(|l| numeric(d.get(l))) {
        return Err("not a Deltarune save".into());
    }
    // A character's block: 54 lines in Chapter 1, 62 later; 1 Kris, 2 Susie, 3 Ralsei, 4 Noelle.
    let (block, members) = if ch1 { (54, 3) } else { (62, 4) };
    let party = named(&DR_MEMBERS[..=members]);
    let p = "Player";
    d.add(p, "Name", 1, Kind::Text);
    d.add(p, "Dark Dollars", 11, Kind::Num);
    for (i, l) in [8, 9, 10].into_iter().enumerate() {
        d.add(p, &format!("Party member {}", i + 1), l, party.clone());
    }
    for (i, (_, who)) in DR_MEMBERS.iter().enumerate().take(members + 1).skip(1) {
        let b = 17 + block * i;
        let sec = format!("Party/{who}");
        for (k, o) in [("HP", 0), ("Max HP", 1), ("Attack", 2), ("Defense", 3), ("Magic", 4), ("Guts", 5), ("Weapon", 6), ("Armor 1", 7), ("Armor 2", 8)] {
            d.add(&sec, k, b + o, Kind::Num);
        }
    }
    if ch1 {
        // Dark World inventory: 13 rows of (item, key item, weapon, armor).
        for (k, o) in [("Items", 0), ("Key items", 1), ("Weapons", 2), ("Armor", 3)] {
            for j in 0..13 {
                d.add(&format!("Inventory/{k}"), &format!("Slot {}", j + 1), 236 + 4 * j + o, Kind::Num);
            }
        }
        d.add("Tension", "TP", 288, Kind::Num);
        d.add("Tension", "Max TP", 289, Kind::Num);
        let lw = "Light World";
        for (k, l) in [("Weapon", 290), ("Armor", 291), ("EXP", 292), ("LV", 293), ("Money", 294), ("HP", 295), ("Max HP", 296), ("AT", 297), ("DF", 298)] {
            d.add(lw, k, l, Kind::Num);
        }
        for i in 0..8 {
            d.add("Light World items", &format!("Slot {}", i + 1), 301 + 2 * i, Kind::Num);
        }
    }
    d.add("Progress", "Plot", n - 2, Kind::Num);
    d.add("Progress", "Room", n - 1, Kind::Num);
    d.add("Progress", "Play time (frames)", n, Kind::Num);
    let note = "Deltarune save: each value is one line of the file. The load menu shows what dr.ini (in the same folder) says; \
                the game reads this file when you continue. Weapons, armor and items are the game's item numbers.";
    Ok(d.finish(if ch1 { DELTARUNE_CH1 } else { DELTARUNE_LATER }, note))
}

/// Either game, if the text is one of their saves.
pub fn parse(text: &str) -> Option<TreeDoc> {
    match line_count(text) {
        549 => parse_undertale(text).ok(),
        3055 | 10318 => parse_deltarune(text).ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic Undertale save: 549 lines, values with a trailing space, CRLF, no final break.
    fn undertale() -> String {
        let mut l: Vec<String> = (0..549).map(|_| "0 ".to_string()).collect();
        l[0] = "Chara".into();
        for (n, v) in [(2, "3"), (3, "28"), (4, "20"), (5, "14"), (6, "3"), (7, "10"), (8, "3"), (9, "4"), (10, "70"), (11, "123"), (12, "9"), (13, "1"), (15, "21"), (29, "13"), (30, "12"), (14, "201"), (548, "12"), (549, "1.80188e+006")] {
            l[n - 1] = format!("{v} ");
        }
        l.join("\r\n")
    }

    #[test]
    fn undertale_values_and_edits() {
        let text = undertale();
        let mut doc = parse(&text).expect("parses");
        assert_eq!(doc.kind_name, UNDERTALE);
        assert_eq!(doc.to_bytes().unwrap(), text.as_bytes());
        assert_eq!(doc.view.pointer("/Character/Name").unwrap(), "Chara");
        assert_eq!(doc.view.pointer("/Character/Gold").unwrap(), 123);
        doc.set("/Route and counters/Killed Flowey", 1.into()).unwrap();
        assert_eq!(parse(&String::from_utf8(doc.to_bytes().unwrap()).unwrap()).unwrap().view.pointer("/Route and counters/Killed Flowey").unwrap(), 1);
        let mut doc = parse(&text).expect("parses");
        assert_eq!(doc.view.pointer("/Items/Slot 1").unwrap(), "Monster Candy");
        assert_eq!(doc.view.pointer("/Items/Slot 2").unwrap(), "Cinnamon Bunny");
        assert_eq!(doc.view.pointer("/Items/Slot 3").unwrap(), "(empty)");
        assert_eq!(doc.view.pointer("/Phone/Slot 1").unwrap(), "Say Hello");
        assert_eq!(doc.view.pointer("/Equipment/Weapon").unwrap(), "Toy Knife");
        assert_eq!(doc.view.pointer("/Progress/Play time (frames)").unwrap(), 1801880);
        assert!(doc.choices("/Items/Slot 1").unwrap().contains(&"Real Knife".to_string()));
        doc.set("/Character/Gold", Value::from(99999)).unwrap();
        doc.set("/Character/Name", Value::from("Frisk")).unwrap();
        doc.set("/Items/Slot 3", Value::from("Legendary Hero")).unwrap();
        doc.set("/Equipment/Weapon", Value::from("Real Knife")).unwrap();
        assert!(doc.set("/Items/Slot 4", Value::from("Not an item")).is_err());
        assert!(doc.set("/Character/Name", Value::from("a\nb")).is_err());
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        let expect = text.replacen("123 \r\n", "99999 \r\n", 1).replacen("Chara\r\n", "Frisk\r\n", 1);
        let lines: Vec<&str> = out.split("\r\n").collect();
        assert_eq!(lines.len(), 549);
        assert_eq!(lines[10], "99999 ");
        assert_eq!(lines[16], "43 ", "Legendary Hero is item 43");
        assert_eq!(lines[28], "52 ", "Real Knife is item 52");
        let mut want: Vec<&str> = expect.split("\r\n").collect();
        want[16] = "43 ";
        want[28] = "52 ";
        assert_eq!(lines, want, "nothing else changes");
        let back = parse(&out).unwrap();
        assert_eq!(back.view.pointer("/Character/Gold").unwrap(), 99999);
        assert_eq!(back.view, doc.view);
    }

    fn deltarune(ch1: bool) -> String {
        let n = if ch1 { 10318 } else { 3055 };
        let block = if ch1 { 54 } else { 62 };
        let mut l: Vec<String> = (0..n).map(|_| "0 ".to_string()).collect();
        l[0] = "VESSEL".into();
        for i in 1..7 {
            l[i] = String::new();
        }
        l[7] = "1 ".into();
        l[8] = "2 ".into();
        l[10] = "500 ".into();
        l[17 + block - 1] = "90 ".into(); // Kris HP
        l[17 + block] = "90 ".into();
        l[17 + 2 * block] = "110 ".into(); // Susie max HP (line 17 + 2 × block + 1)
        if ch1 {
            l[235] = "3 ".into(); // first item
            l[287] = "145.75 ".into(); // TP
        }
        l[n - 1] = "1.56599e+06 ".into();
        l.join("\r\n")
    }

    #[test]
    fn deltarune_values_and_edits() {
        for ch1 in [true, false] {
            let text = deltarune(ch1);
            let mut doc = parse(&text).expect("parses");
            assert_eq!(doc.kind_name, if ch1 { DELTARUNE_CH1 } else { DELTARUNE_LATER });
            assert_eq!(doc.to_bytes().unwrap(), text.as_bytes());
            assert_eq!(doc.view.pointer("/Player/Name").unwrap(), "VESSEL");
            assert_eq!(doc.view.pointer("/Player/Dark Dollars").unwrap(), 500);
            assert_eq!(doc.view.pointer("/Player/Party member 2").unwrap(), "Susie");
            assert_eq!(doc.view.pointer("/Party/Kris/HP").unwrap(), 90);
            assert_eq!(doc.view.pointer("/Party/Susie/Max HP").unwrap(), 110);
            assert_eq!(doc.view.pointer("/Party/Noelle").is_some(), !ch1);
            assert_eq!(doc.view.pointer("/Inventory/Items/Slot 1").is_some(), ch1);
            if ch1 {
                assert_eq!(doc.view.pointer("/Inventory/Items/Slot 1").unwrap(), 3);
                assert_eq!(doc.view.pointer("/Tension/TP").unwrap().as_f64(), Some(145.75));
            }
            doc.set("/Player/Dark Dollars", Value::from(501)).unwrap();
            doc.set("/Player/Party member 3", Value::from("Ralsei")).unwrap();
            let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
            let a: Vec<&str> = out.split("\r\n").collect();
            let b: Vec<&str> = text.split("\r\n").collect();
            assert_eq!(a.len(), b.len());
            let diff: Vec<usize> = (0..a.len()).filter(|&i| a[i] != b[i]).collect();
            assert_eq!(diff, vec![9, 10]);
            assert_eq!((a[9], a[10]), ("3 ", "501 "));
        }
    }

    #[test]
    fn other_text_is_not_a_save() {
        assert!(parse("hello\r\nworld").is_none());
        let not_ut: String = (0..549).map(|_| "word").collect::<Vec<_>>().join("\r\n");
        assert!(parse(&not_ut).is_none());
    }

    /// Save files under `dir` (file0/file8/file9, filech*).
    fn saves_in(dir: &std::path::Path, out: &mut Vec<String>) {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if p.is_dir() {
                saves_in(&p, out);
            } else if ["file0", "file8", "file9"].contains(&name.as_str()) || name.starts_with("filech") {
                out.push(p.to_string_lossy().to_string());
            }
        }
    }

    /// Real saves: `UNDERTALE_SAVES` / `DELTARUNE_SAVES` = files or folders separated by `|`.
    #[test]
    #[ignore]
    fn research_real_saves() {
        for var in ["UNDERTALE_SAVES", "DELTARUNE_SAVES"] {
            let Ok(list) = std::env::var(var) else { continue };
            let mut paths = vec![];
            for p in list.split('|').filter(|s| !s.is_empty()) {
                if std::path::Path::new(p).is_dir() {
                    saves_in(std::path::Path::new(p), &mut paths);
                } else {
                    paths.push(p.to_string());
                }
            }
            for path in &paths {
                let raw = std::fs::read(path).unwrap();
                let text = String::from_utf8(raw.clone()).unwrap();
                let mut doc = parse(&text).unwrap_or_else(|| panic!("{path} didn't parse"));
                assert_eq!(doc.to_bytes().unwrap(), raw, "{path}: round trip");
                let gold = if doc.kind_name == UNDERTALE { "/Character/Gold" } else { "/Player/Dark Dollars" };
                let g = doc.view.pointer(gold).and_then(Value::as_i64).unwrap();
                let before = doc.view.clone();
                doc.set(gold, Value::from(g + 1)).unwrap();
                let back = parse(std::str::from_utf8(&doc.to_bytes().unwrap()).unwrap()).unwrap();
                assert_eq!(back.view.pointer(gold).and_then(Value::as_i64), Some(g + 1), "{path}");
                let mut b = back.view.clone();
                *b.pointer_mut(gold).unwrap() = Value::from(g);
                assert_eq!(b, before, "{path}: nothing else changes");
                eprintln!("{path}: {} gold {g}", doc.kind_name);
            }
        }
    }
}
