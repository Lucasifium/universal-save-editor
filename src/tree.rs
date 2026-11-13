//! Generic editable tree documents.
//!
//! Every structured binary/text format works the same way: a format reader builds a
//! friendly display tree (`serde_json::Value`) and registers each editable scalar as a
//! numbered *leaf* that the format's `Backend` knows how to rewrite. Saving hands the
//! backend only the leaves that changed, so anything the editor doesn't understand is
//! written back untouched.

use serde_json::{Map, Number, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub trait Backend: std::fmt::Debug + Send + Sync {
    /// Produce the file bytes with `changes` (leaf id, new value) applied.
    /// With no changes this must reproduce the original data.
    fn write(&self, view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String>;
    /// Adjust a new value to what the format can actually store (e.g. round to the
    /// nearest 32-bit float), so the screen shows exactly what gets saved.
    fn normalize(&self, _leaf: usize, v: Value) -> Value {
        v
    }
    /// Validate a new value before it's accepted (ranges, types).
    fn check(&self, _leaf: usize, _v: &Value) -> Result<(), String> {
        Ok(())
    }
    /// Number of extra places kept in sync with a leaf (e.g. Ren'Py rollback history).
    fn linked(&self, _leaf: usize) -> usize {
        0
    }
    /// Other values that follow from a change (e.g. a Pokémon's experience and stats
    /// after its level is edited), as (pointer, new value). `view` already holds the
    /// new value of `leaf`.
    fn derived(&self, _leaf: usize, _view: &Value) -> Vec<(String, Value)> {
        vec![]
    }
    /// A fixed list of allowed values (shown as a drop-down), if the field has one.
    fn choices(&self, _leaf: usize, _view: &Value) -> Option<Arc<Vec<String>>> {
        None
    }
    /// Does changing this leaf change the file's layout (e.g. add an item)? Then the saved
    /// file reads back as a different tree, and is checked with `TreeDoc::agrees_with`.
    fn reshapes(&self, _leaf: usize) -> bool {
        false
    }
    /// Extra decoded data a game screen draws from (e.g. a Terraria world's tile grid),
    /// too big for the value tree.
    fn attachment(&self) -> Option<Arc<dyn std::any::Any + Send + Sync>> {
        None
    }
    /// Why a value that isn't editable can't be, when the format knows (shown on hover).
    fn read_only_reason(&self, _view: &Value, _ptr: &str) -> Option<String> {
        None
    }
    fn clone_box(&self) -> Box<dyn Backend>;
}

impl Clone for Box<dyn Backend> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[derive(Clone, Debug)]
pub struct TreeDoc {
    /// Friendly display tree shown/edited in the UI.
    pub view: Value,
    original: Value,
    /// JSON pointer in `view` → leaf id. Pointers not listed here are read-only.
    pub editable: HashMap<String, usize>,
    /// Short format name for the UI ("Ren'Py save", "Minecraft NBT", …).
    pub kind_name: String,
    /// Top-level sections hidden unless "Advanced" is on.
    pub advanced: Vec<String>,
    /// Something the player should know about this format (shown once on open).
    pub note: Option<String>,
    backend: Box<dyn Backend>,
    /// Values changed as a consequence of the last `set` (pointer, before, after).
    derived_log: Vec<(String, Value, Value)>,
    /// Ranges already worked out by `range` (asking some formats is slow).
    ranges: RangeCache,
}

#[derive(Debug, Default)]
struct RangeCache(std::sync::Mutex<HashMap<String, Option<(Value, Value)>>>);

impl Clone for RangeCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl TreeDoc {
    pub fn new(view: Value, editable: HashMap<String, usize>, kind_name: impl Into<String>, backend: Box<dyn Backend>) -> Self {
        Self { original: view.clone(), view, editable, kind_name: kind_name.into(), advanced: vec![], note: None, backend, derived_log: vec![], ranges: RangeCache::default() }
    }

    /// See `Backend::attachment`.
    pub fn attachment(&self) -> Option<Arc<dyn std::any::Any + Send + Sync>> {
        self.backend.attachment()
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    pub fn with_advanced(mut self, advanced: Vec<String>) -> Self {
        self.advanced = advanced;
        self
    }

    pub fn view_and_editable(&mut self) -> (&mut Value, &HashMap<String, usize>) {
        (&mut self.view, &self.editable)
    }

    /// Set a value. The same underlying data can appear at several places in the view
    /// (shared objects); all of them are updated so the screen matches what's saved.
    /// Returns the old value and how many *other* places changed with it, or an
    /// error if the format can't store this value (e.g. out of range).
    pub fn set(&mut self, ptr: &str, v: Value) -> Result<Option<(Value, usize)>, String> {
        let leaf = self.editable.get(ptr).copied();
        let v = match leaf {
            Some(leaf) => {
                let v = self.backend.normalize(leaf, v);
                self.backend.check(leaf, &v)?;
                v
            }
            None => v,
        };
        if leaf.is_some_and(|l| self.backend.reshapes(l)) {
            if let Ok(mut c) = self.ranges.0.lock() {
                c.clear();
            }
        }
        let Some(target) = self.view.pointer_mut(ptr) else { return Ok(None) };
        let old = std::mem::replace(target, v.clone());
        let mut aliases = 0;
        if let Some(leaf) = leaf {
            for (p, &l) in &self.editable {
                if l == leaf && p != ptr {
                    if let Some(t) = self.view.pointer_mut(p) {
                        *t = v.clone();
                        aliases += 1;
                    }
                }
            }
        }
        self.derived_log.clear();
        if let Some(leaf) = leaf {
            for (p, nv) in self.backend.derived(leaf, &self.view) {
                // The edited value itself can be adjusted too (e.g. a bag item moving up
                // to the first free slot); `set` callers read the stored value back.
                if p == ptr {
                    if let Some(t) = self.view.pointer_mut(&p) {
                        *t = nv;
                    }
                    continue;
                }
                if let Some(t) = self.view.pointer_mut(&p) {
                    if *t != nv {
                        let before = std::mem::replace(t, nv.clone());
                        self.derived_log.push((p, before, nv));
                    }
                }
            }
        }
        Ok(Some((old, aliases)))
    }

    /// Set a value exactly, without working out consequences (used by undo/redo).
    pub fn set_plain(&mut self, ptr: &str, v: Value) {
        if let Some(t) = self.view.pointer_mut(ptr) {
            *t = v;
        }
    }

    /// The values that changed along with the last `set`, as (pointer, before, after).
    pub fn take_derived(&mut self) -> Vec<(String, Value, Value)> {
        std::mem::take(&mut self.derived_log)
    }

    /// Allowed values for a field shown as a drop-down, if it has a fixed list.
    pub fn choices(&self, ptr: &str) -> Option<Arc<Vec<String>>> {
        let leaf = *self.editable.get(ptr)?;
        self.backend.choices(leaf, &self.view)
    }

    /// The smallest and largest value this number field accepts, found by asking the format
    /// to check values far outside any range (formats report "must be between A and B").
    /// `None` when the field is unbounded, not a number, or the format gives no range.
    pub fn range(&self, ptr: &str) -> Option<(Value, Value)> {
        if let Some(hit) = self.ranges.0.lock().ok().and_then(|c| c.get(ptr).cloned()) {
            return hit;
        }
        let found = self.probe_range(ptr);
        if let Ok(mut c) = self.ranges.0.lock() {
            c.insert(ptr.to_string(), found.clone());
        }
        found
    }

    fn probe_range(&self, ptr: &str) -> Option<(Value, Value)> {
        let leaf = *self.editable.get(ptr)?;
        let cur = self.view.pointer(ptr)?;
        let Value::Number(n) = cur else { return None };
        let whole = n.as_i64().is_some() || n.as_u64().is_some();
        let probes: [Value; 2] = if whole { [Value::from(i64::MAX), Value::from(i64::MIN)] } else { [num(1e300), num(-1e300)] };
        let err = probes.iter().find_map(|p| self.backend.check(leaf, p).err())?;
        let (lo, hi) = parse_between(&err, whole)?;
        // Trust the message only when the format really accepts both ends.
        (self.backend.check(leaf, &lo).is_ok() && self.backend.check(leaf, &hi).is_ok()).then_some((lo, hi))
    }

    /// Why this value can't be edited, as specifically as is known.
    pub fn read_only_reason(&self, ptr: &str) -> String {
        if self.editable.contains_key(ptr) {
            return String::new();
        }
        if let Some(r) = self.backend.read_only_reason(&self.view, ptr) {
            return r;
        }
        read_only_reason_for_key(ptr, &self.kind_name)
    }

    #[cfg(test)]
    pub fn linked_count(&self, ptr: &str) -> usize {
        self.editable.get(ptr).map_or(0, |&l| self.backend.linked(l))
    }

    /// Has an edit that changes the file's layout been made?
    pub fn reshaped(&self) -> bool {
        self.editable.iter().any(|(ptr, &leaf)| self.backend.reshapes(leaf) && self.view.pointer(ptr) != self.original.pointer(ptr))
    }

    /// After a layout-changing save: every ordinary value that `other` (the file read back)
    /// still has must be what was edited here.
    pub fn agrees_with(&self, other: &TreeDoc) -> bool {
        self.kind_name == other.kind_name
            && self.editable.iter().all(|(ptr, &leaf)| {
                self.backend.reshapes(leaf) || other.view.pointer(ptr).is_none_or(|v| Some(v) == self.view.pointer(ptr))
            })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        let mut seen = HashSet::new();
        let mut changes: Vec<(usize, Value)> = Vec::new();
        for (ptr, &leaf) in &self.editable {
            let (Some(cur), Some(orig)) = (self.view.pointer(ptr), self.original.pointer(ptr)) else { continue };
            if cur != orig && seen.insert(leaf) {
                changes.push((leaf, cur.clone()));
            }
        }
        changes.sort_by_key(|(l, _)| *l);
        self.backend.write(&self.view, &changes)
    }
}

// ----------------------------------------------------------------- view helpers

pub fn esc(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

pub fn child(ptr: &str, key: &str) -> String {
    format!("{ptr}/{}", esc(key))
}

/// Insert under a key that isn't taken yet ("name", "name (2)", …); returns the key used.
pub fn insert_unique(map: &mut Map<String, Value>, key: &str, v: Value) -> String {
    let mut k = key.to_string();
    let mut n = 2;
    while map.contains_key(&k) {
        k = format!("{key} ({n})");
        n += 1;
    }
    map.insert(k.clone(), v);
    k
}

/// The key `insert_unique` would choose, without inserting.
pub fn unique_key(map: &Map<String, Value>, key: &str) -> String {
    let mut k = key.to_string();
    let mut n = 2;
    while map.contains_key(&k) {
        k = format!("{key} ({n})");
        n += 1;
    }
    k
}

/// Text from text formats (XML, INI) shown with a useful editor: plain whole numbers,
/// simple decimals and true/false get number/switch editors; anything else stays text
/// so it's written back exactly as typed.
pub fn text_to_value(s: &str) -> Value {
    let t = s;
    match t {
        "true" | "True" | "TRUE" => return Value::Bool(true),
        "false" | "False" | "FALSE" => return Value::Bool(false),
        _ => {}
    }
    let digits = t.strip_prefix('-').unwrap_or(t);
    let plain_int = !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) && (digits == "0" || !digits.starts_with('0')) && t != "-0";
    if plain_int {
        if let Ok(i) = t.parse::<i64>() {
            return Value::from(i);
        }
    }
    if let Some((a, b)) = digits.split_once('.') {
        let ok = !a.is_empty() && !b.is_empty() && a.bytes().chain(b.bytes()).all(|c| c.is_ascii_digit()) && (a == "0" || !a.starts_with('0'));
        if ok {
            if let Ok(n) = serde_json::from_str::<Number>(t) {
                return Value::Number(n);
            }
        }
    }
    Value::String(t.to_string())
}

/// Format an edited value back as text, matching the original's true/false style.
pub fn value_to_text(orig: &str, v: &Value) -> String {
    match v {
        Value::Bool(b) => {
            let s = if *b { "true" } else { "false" };
            if orig.chars().all(|c| c.is_ascii_uppercase()) && !orig.is_empty() {
                s.to_uppercase()
            } else if orig.starts_with(|c: char| c.is_ascii_uppercase()) {
                let mut c = s.chars();
                c.next().map(|f| f.to_ascii_uppercase().to_string() + c.as_str()).unwrap_or_default()
            } else {
                s.to_string()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// "must be between 0 and 255" (also "Must be a whole number between 0 and 8,388,608") → (0, 255).
pub fn parse_between(msg: &str, whole: bool) -> Option<(Value, Value)> {
    let lower = msg.to_lowercase();
    let rest: Vec<char> = lower[lower.find("between")? + "between".len()..].chars().collect();
    // Numeric tokens with the text before each.
    let mut toks: Vec<(String, String)> = Vec::new();
    let (mut i, mut gap) = (0, String::new());
    while i < rest.len() && toks.len() < 2 {
        let c = rest[i];
        let neg = c == '-' && rest.get(i + 1).is_some_and(|d| d.is_ascii_digit()) && (i == 0 || rest[i - 1].is_whitespace());
        if c.is_ascii_digit() || neg {
            let mut t = String::from(c);
            i += 1;
            while i < rest.len() && (rest[i].is_ascii_digit() || (rest[i] == ',' && rest.get(i + 1).is_some_and(|d| d.is_ascii_digit())) || (rest[i] == '.' && rest.get(i + 1).is_some_and(|d| d.is_ascii_digit()) && !t.contains('.'))) {
                t.push(rest[i]);
                i += 1;
            }
            toks.push((gap.trim().to_string(), t.replace(',', "")));
            gap.clear();
        } else {
            gap.push(c);
            i += 1;
        }
    }
    if toks.len() != 2 || !matches!(toks[1].0.as_str(), "and" | "to" | "-" | "\u{2013}") {
        return None;
    }
    let conv = |t: &str| -> Option<Value> {
        if whole {
            t.parse::<i64>().ok().map(Value::from).or_else(|| t.parse::<u64>().ok().map(Value::from))
        } else {
            t.parse::<f64>().ok().map(num)
        }
    };
    let (lo, hi) = (conv(&toks[0].1)?, conv(&toks[1].1)?);
    (lo.as_f64()? <= hi.as_f64()?).then_some((lo, hi))
}

/// A best-effort explanation for a value the editor won't change, from its name.
fn read_only_reason_for_key(ptr: &str, kind: &str) -> String {
    let key = ptr.rsplit('/').next().unwrap_or("").to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| key.contains(w));
    let why = if has(&["checksum", "crc", "hash", "signature", "digest", "md5", "sha1", "sha256"]) {
        "a checksum or signature. The editor recalculates it when you save, so it can't be typed in"
    } else if has(&["magic", "header", "version", "format"]) {
        "part of the file's header. Changing it could make the game refuse the save"
    } else if has(&["size", "length", "len", "count", "offset", "pointer", "ptr", "index", "num_"]) {
        "a size, count or position the file uses to find its other data. It follows from the rest of the save and is kept in step automatically"
    } else if has(&["time", "date", "stamp", "playtime", "play_time"]) {
        "a timestamp or play-time counter the game manages itself"
    } else {
        ""
    };
    if why.is_empty() {
        format!("Read-only: this {kind} value isn't stored in a form the editor can safely rewrite")
    } else {
        format!("Read-only: this looks like {why}")
    }
}

/// Integer value of a JSON value if it's a whole number that fits in `min..=max`.
pub fn int_in(v: &Value, min: i128, max: i128, what: &str) -> Result<i128, String> {
    let n = match v {
        Value::Number(n) => n.as_i64().map(i128::from).or_else(|| n.as_u64().map(i128::from)),
        Value::Bool(b) => Some(i128::from(*b)),
        _ => None,
    }
    .ok_or_else(|| format!("{what} must be a whole number"))?;
    if n < min || n > max {
        return Err(format!("{what} must be between {min} and {max}"));
    }
    Ok(n)
}

/// A 32-bit float shown with the shortest digits that read back as the same float
/// (0.1, not 0.10000000149011612).
pub fn f32_value(f: f32) -> Value {
    format!("{f}").parse::<f64>().ok().map_or_else(|| num(f as f64), num)
}

/// Round a number to what a 32-bit float field stores (and show it the way such a
/// field is shown when the file is opened); other values pass through.
pub fn round_f32(v: Value) -> Value {
    match v.as_f64() {
        Some(f) if f.is_finite() => f32_value(f as f32),
        _ => v,
    }
}

pub fn num(f: f64) -> Value {
    Number::from_f64(f).map(Value::Number).unwrap_or_else(|| Value::String(f.to_string()))
}

/// A byte-range patcher: collect (start, end, replacement) and apply them in one pass.
#[derive(Default)]
pub struct Patches(Vec<(usize, usize, Vec<u8>)>);

impl Patches {
    pub fn add(&mut self, start: usize, end: usize, bytes: Vec<u8>) {
        self.0.push((start, end, bytes));
    }
    pub fn apply(mut self, raw: &[u8]) -> Result<Vec<u8>, String> {
        self.0.sort_by_key(|p| p.0);
        let mut out = Vec::with_capacity(raw.len() + 64);
        let mut pos = 0;
        for (s, e, b) in self.0 {
            if s < pos || e < s || e > raw.len() {
                return Err("overlapping edits".into());
            }
            out.extend_from_slice(&raw[pos..s]);
            out.extend_from_slice(&b);
            pos = e;
        }
        out.extend_from_slice(&raw[pos..]);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_values() {
        assert_eq!(text_to_value("1500"), Value::from(1500));
        assert_eq!(text_to_value("-7"), Value::from(-7));
        assert_eq!(text_to_value("007"), Value::from("007"), "leading zeros stay text");
        assert_eq!(text_to_value("1e5"), Value::from("1e5"));
        assert!(text_to_value("2.50").is_number());
        assert_eq!(text_to_value("True"), Value::Bool(true));
        assert_eq!(value_to_text("True", &Value::Bool(false)), "False");
        assert_eq!(value_to_text("TRUE", &Value::Bool(false)), "FALSE");
        assert_eq!(value_to_text("true", &Value::Bool(false)), "false");
    }

    #[derive(Debug, Clone)]
    struct Capped;
    impl Backend for Capped {
        fn write(&self, _view: &Value, _changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
            Ok(vec![])
        }
        fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
            match leaf {
                0 => int_in(v, 0, 99, "Level").map(|_| ()),
                1 => Ok(()),
                _ => Err("nope".into()),
            }
        }
        fn read_only_reason(&self, _view: &Value, ptr: &str) -> Option<String> {
            (ptr == "/special").then(|| "Derived from the level".to_string())
        }
        fn clone_box(&self) -> Box<dyn Backend> {
            Box::new(self.clone())
        }
    }

    #[test]
    fn field_ranges_are_found_by_probing() {
        let view = serde_json::json!({"level": 5, "gold": 7, "name": "x", "special": 3, "crc32": 1});
        let editable: HashMap<String, usize> = [("/level".to_string(), 0), ("/gold".to_string(), 1), ("/name".to_string(), 2)].into();
        let doc = TreeDoc::new(view, editable, "Test", Box::new(Capped));
        assert_eq!(doc.range("/level"), Some((Value::from(0), Value::from(99))));
        assert_eq!(doc.range("/level"), Some((Value::from(0), Value::from(99))), "cached answer is the same");
        assert_eq!(doc.range("/gold"), None, "an unbounded field has no range");
        assert_eq!(doc.range("/name"), None, "text has no range");
        assert_eq!(doc.range("/nothing"), None);
        assert_eq!(doc.read_only_reason("/special"), "Derived from the level");
        assert!(doc.read_only_reason("/crc32").contains("checksum"));
        assert!(doc.read_only_reason("/other").starts_with("Read-only"));
        assert_eq!(doc.read_only_reason("/level"), "");
    }

    #[test]
    fn ranges_parse_from_messages() {
        assert_eq!(parse_between("must be between 0 and 255", true), Some((Value::from(0), Value::from(255))));
        assert_eq!(parse_between("Must be a whole number between 0 and 8,388,608", true), Some((Value::from(0), Value::from(8388608))));
        assert_eq!(parse_between("HP must be between -5 and 5", true), Some((Value::from(-5), Value::from(5))));
        assert_eq!(parse_between("temperature must be between absolute zero and 9,999 K", false), None);
        assert_eq!(parse_between("unexpected data between the save's blocks", true), None);
        assert_eq!(parse_between("must be between 0.5 and 2.5", false), Some((num(0.5), num(2.5))));
    }

    #[test]
    fn patches_apply() {
        let mut p = Patches::default();
        p.add(6, 11, b"there".to_vec());
        p.add(0, 5, b"Hi".to_vec());
        assert_eq!(p.apply(b"hello world!").unwrap(), b"Hi there!");
    }
}
