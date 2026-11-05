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
}

impl TreeDoc {
    pub fn new(view: Value, editable: HashMap<String, usize>, kind_name: impl Into<String>, backend: Box<dyn Backend>) -> Self {
        Self { original: view.clone(), view, editable, kind_name: kind_name.into(), advanced: vec![], note: None, backend, derived_log: vec![] }
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

    #[test]
    fn patches_apply() {
        let mut p = Patches::default();
        p.add(6, 11, b"there".to_vec());
        p.add(0, 5, b"Hi".to_vec());
        assert_eq!(p.apply(b"hello world!").unwrap(), b"Hi there!");
    }
}
