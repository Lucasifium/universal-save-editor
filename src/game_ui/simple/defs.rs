//! Game screens described in data files: `data/games/*.toml`, one file per game, embedded at
//! build time (see `build.rs`) and turned into the same `Spec` the Rust tables use. The format
//! is documented in `data/games/README.md`.
//!
//! A file that does not parse is skipped (a screen is never worth a crash); the tests fail on
//! it, so it cannot ship.

use super::{List, Max, Section, Shape, Spec, Table};
use serde::{Deserialize, Serialize};

include!(concat!(env!("OUT_DIR"), "/game_definitions.rs"));

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct Def {
    pub name: String,
    pub tab: String,
    pub accent: [u8; 3],
    /// The kind name the reader gives the save; absent for a plain JSON save.
    #[serde(default)]
    pub kind: Option<String>,
    pub markers: Vec<String>,
    pub subtitle: String,
    /// `[caption, pointer]` pairs.
    #[serde(default)]
    pub tiles: Vec<[String; 2]>,
    #[serde(default)]
    pub sections: Vec<SectionDef>,
    #[serde(default)]
    pub lists: Vec<ListDef>,
    #[serde(default)]
    pub tables: Vec<TableDef>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SectionDef {
    pub title: String,
    /// `[label, pointer]` pairs.
    pub rows: Vec<[String; 2]>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ListDef {
    pub title: String,
    pub path: String,
    /// `array`, `map` or `records`.
    pub shape: String,
    /// `array`: the key naming each item.
    #[serde(default)]
    pub id_key: Option<String>,
    /// `array`: the count's key first, then keys of copies kept equal to it.
    #[serde(default)]
    pub qty_keys: Vec<String>,
    /// `records`: the key holding each record's count.
    #[serde(default)]
    pub qty_key: Option<String>,
    /// `[id, name]` pairs.
    #[serde(default)]
    pub names: Vec<[String; 2]>,
    #[serde(default)]
    pub max: Option<MaxDef>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MaxDef {
    pub ids: Vec<String>,
    pub cap: i64,
    /// `[id, cap]` pairs for ids whose cap differs from `cap`.
    #[serde(default)]
    pub caps: Vec<(String, i64)>,
    #[serde(default)]
    pub estimate: bool,
    pub about: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TableDef {
    pub title: String,
    pub path: String,
    #[serde(default)]
    pub name: Vec<String>,
    /// `[heading, pointer]` pairs.
    pub cols: Vec<[String; 2]>,
    #[serde(default)]
    pub only: Vec<String>,
    /// `[text, name]` pairs.
    #[serde(default)]
    pub names: Vec<[String; 2]>,
}

/// Leaks a string: the specs live for the whole run, as the Rust tables do.
fn leak(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

fn leak_strs(v: &[String]) -> &'static [&'static str] {
    Box::leak(v.iter().map(|s| leak(s)).collect::<Vec<_>>().into_boxed_slice())
}

fn leak_pairs(v: &[[String; 2]]) -> &'static [(&'static str, &'static str)] {
    Box::leak(v.iter().map(|[a, b]| (leak(a), leak(b))).collect::<Vec<_>>().into_boxed_slice())
}

impl Def {
    /// Parses one definition file and checks it is complete.
    pub fn parse(text: &str) -> Result<Def, String> {
        let d: Def = toml_edit::de::from_str(text).map_err(|e| e.to_string())?;
        d.check()?;
        Ok(d)
    }

    fn check(&self) -> Result<(), String> {
        let bad = |what: &str| Err(format!("{}: {what}", self.name));
        if self.name.is_empty() || self.tab.is_empty() || self.subtitle.is_empty() {
            return bad("name, tab and subtitle are required");
        }
        if self.markers.iter().any(|m| !m.starts_with('/')) {
            return bad("markers must be pointers starting with /");
        }
        let pointers = self.tiles.iter().map(|t| &t[1]).chain(self.sections.iter().flat_map(|s| s.rows.iter().map(|r| &r[1])));
        if pointers.into_iter().any(|p| !p.starts_with('/')) {
            return bad("every tile and row pointer must start with /");
        }
        for l in &self.lists {
            let ok = match l.shape.as_str() {
                "array" => l.id_key.is_some() && !l.qty_keys.is_empty() && l.qty_key.is_none(),
                "map" => l.id_key.is_none() && l.qty_keys.is_empty() && l.qty_key.is_none(),
                "records" => l.qty_key.is_some() && l.id_key.is_none() && l.qty_keys.is_empty(),
                _ => false,
            };
            if !ok {
                return bad(&format!("list \"{}\": shape must be array (id_key, qty_keys), map, or records (qty_key)", l.title));
            }
        }
        Ok(())
    }

    /// The table-driven screen this describes.
    pub fn into_spec(self) -> Spec {
        Spec {
            name: leak(&self.name),
            tab: leak(&self.tab),
            accent: (self.accent[0], self.accent[1], self.accent[2]),
            kind: self.kind.as_deref().map(leak),
            markers: leak_strs(&self.markers),
            subtitle: leak(&self.subtitle),
            tiles: leak_pairs(&self.tiles),
            sections: Box::leak(self.sections.iter().map(|s| Section { title: leak(&s.title), rows: leak_pairs(&s.rows) }).collect::<Vec<_>>().into_boxed_slice()),
            lists: Box::leak(
                self.lists
                    .iter()
                    .map(|l| List {
                        title: leak(&l.title),
                        path: leak(&l.path),
                        shape: match l.shape.as_str() {
                            "array" => Shape::Array { id_key: leak(l.id_key.as_deref().unwrap_or("")), qty_keys: leak_strs(&l.qty_keys) },
                            "map" => Shape::Map,
                            _ => Shape::Records { qty_key: leak(l.qty_key.as_deref().unwrap_or("")) },
                        },
                        names: leak_pairs(&l.names),
                        max: l.max.as_ref().map(|m| Max {
                            ids: leak_strs(&m.ids),
                            cap: m.cap,
                            caps: Box::leak(m.caps.iter().map(|(i, c)| (leak(i), *c)).collect::<Vec<_>>().into_boxed_slice()),
                            estimate: m.estimate,
                            about: leak(&m.about),
                        }),
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
            tables: Box::leak(
                self.tables
                    .iter()
                    .map(|t| Table { title: leak(&t.title), path: leak(&t.path), name: leak_strs(&t.name), cols: leak_pairs(&t.cols), only: leak_strs(&t.only), names: leak_pairs(&t.names) })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        }
    }

    /// The definition of a Rust-table spec (used to move games into data files and to check
    /// that a converted game is unchanged).
    #[cfg(test)]
    pub fn from_spec(s: &Spec) -> Def {
        let pairs = |v: &[(&str, &str)]| v.iter().map(|(a, b)| [a.to_string(), b.to_string()]).collect::<Vec<_>>();
        let strs = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        Def {
            name: s.name.into(),
            tab: s.tab.into(),
            accent: [s.accent.0, s.accent.1, s.accent.2],
            kind: s.kind.map(str::to_string),
            markers: strs(s.markers),
            subtitle: s.subtitle.into(),
            tiles: pairs(s.tiles),
            sections: s.sections.iter().map(|x| SectionDef { title: x.title.into(), rows: pairs(x.rows) }).collect(),
            lists: s
                .lists
                .iter()
                .map(|l| {
                    let (shape, id_key, qty_keys, qty_key) = match &l.shape {
                        Shape::Array { id_key, qty_keys } => ("array", Some(id_key.to_string()), strs(qty_keys), None),
                        Shape::Map => ("map", None, vec![], None),
                        Shape::Records { qty_key } => ("records", None, vec![], Some(qty_key.to_string())),
                    };
                    ListDef {
                        title: l.title.into(),
                        path: l.path.into(),
                        shape: shape.into(),
                        id_key,
                        qty_keys,
                        qty_key,
                        names: pairs(l.names),
                        max: l.max.as_ref().map(|m| MaxDef { ids: strs(m.ids), cap: m.cap, caps: m.caps.iter().map(|(i, c)| (i.to_string(), *c)).collect(), estimate: m.estimate, about: m.about.into() }),
                    }
                })
                .collect(),
            tables: s.tables.iter().map(|t| TableDef { title: t.title.into(), path: t.path.into(), name: strs(t.name), cols: pairs(t.cols), only: strs(t.only), names: pairs(t.names) }).collect(),
        }
    }

    /// The definition as a data file, written the way the files in data/games are.
    #[cfg(test)]
    pub fn to_toml(&self) -> String {
        use toml_edit::Value;
        let q = |s: &str| Value::from(s).to_string().trim().to_string();
        let list = |v: &[String]| format!("[{}]", v.iter().map(|s| q(s)).collect::<Vec<_>>().join(", "));
        let pairs = |key: &str, v: &[[String; 2]]| -> String {
            if v.is_empty() {
                return String::new();
            }
            let mut o = format!("{key} = [\n");
            for [a, b] in v {
                o.push_str(&format!("    [{}, {}],\n", q(a), q(b)));
            }
            o.push_str("]\n");
            o
        };
        let mut o = format!("name = {}\ntab = {}\naccent = [{}, {}, {}]\n", q(&self.name), q(&self.tab), self.accent[0], self.accent[1], self.accent[2]);
        if let Some(k) = &self.kind {
            o.push_str(&format!("kind = {}\n", q(k)));
        }
        o.push_str(&format!("markers = {}\nsubtitle = {}\n", list(&self.markers), q(&self.subtitle)));
        o.push_str(&pairs("tiles", &self.tiles));
        for s in &self.sections {
            o.push_str(&format!("\n[[sections]]\ntitle = {}\n{}", q(&s.title), pairs("rows", &s.rows)));
        }
        for l in &self.lists {
            o.push_str(&format!("\n[[lists]]\ntitle = {}\npath = {}\nshape = {}\n", q(&l.title), q(&l.path), q(&l.shape)));
            if let Some(k) = &l.id_key {
                o.push_str(&format!("id_key = {}\n", q(k)));
            }
            if !l.qty_keys.is_empty() {
                o.push_str(&format!("qty_keys = {}\n", list(&l.qty_keys)));
            }
            if let Some(k) = &l.qty_key {
                o.push_str(&format!("qty_key = {}\n", q(k)));
            }
            o.push_str(&pairs("names", &l.names));
            if let Some(m) = &l.max {
                o.push_str(&format!("\n[lists.max]\nids = {}\ncap = {}\n", list(&m.ids), m.cap));
                if !m.caps.is_empty() {
                    o.push_str(&format!("caps = [{}]\n", m.caps.iter().map(|(i, c)| format!("[{}, {c}]", q(i))).collect::<Vec<_>>().join(", ")));
                }
                if m.estimate {
                    o.push_str("estimate = true\n");
                }
                o.push_str(&format!("about = {}\n", q(&m.about)));
            }
        }
        for t in &self.tables {
            o.push_str(&format!("\n[[tables]]\ntitle = {}\npath = {}\n", q(&t.title), q(&t.path)));
            if !t.name.is_empty() {
                o.push_str(&format!("name = {}\n", list(&t.name)));
            }
            o.push_str(&pairs("cols", &t.cols));
            if !t.only.is_empty() {
                o.push_str(&format!("only = {}\n", list(&t.only)));
            }
            o.push_str(&pairs("names", &t.names));
        }
        o
    }
}

/// The games described in data files, in file-name order. A file that fails to parse is left
/// out (and named on stderr); `every_definition_parses` keeps that from shipping.
pub fn load() -> Vec<Spec> {
    GAME_DEFINITIONS
        .iter()
        .filter_map(|(file, text)| match Def::parse(text) {
            Ok(d) => Some(d.into_spec()),
            Err(e) => {
                eprintln!("game definition {file}: {e}");
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_definition_parses() {
        for (file, text) in GAME_DEFINITIONS {
            if let Err(e) = Def::parse(text) {
                panic!("data/games/{file}: {e}");
            }
        }
    }

    #[test]
    fn every_definition_is_in_use_and_has_its_own_name() {
        assert!(GAME_DEFINITIONS.len() >= 13, "data/games is missing from the build");
        assert_eq!(load().len(), GAME_DEFINITIONS.len());
        let data: HashSet<&str> = load().iter().map(|s| s.name).collect();
        for d in &data {
            assert_eq!(super::super::all().iter().filter(|s| s.name == *d).count(), 1, "another screen is also called \"{d}\"");
        }
    }

    /// Definitions are checked after the Rust tables, so a Rust screen must never recognise
    /// the saves of a data-file game first: no Rust screen of the same kind may need only a
    /// subset of its markers (or the reverse, which would let the data game steal that save).
    #[test]
    fn definitions_do_not_overlap_the_rust_screens() {
        let data: HashSet<&str> = load().iter().map(|s| s.name).collect();
        let all = super::super::all();
        for d in all.iter().filter(|s| data.contains(s.name)) {
            for r in all.iter().filter(|s| !data.contains(s.name) && s.kind == d.kind) {
                let r_in_d = r.markers.iter().all(|m| d.markers.contains(m));
                let d_in_r = d.markers.iter().all(|m| r.markers.contains(m));
                assert!(!r_in_d && !d_in_r, "\"{}\" and \"{}\" recognise overlapping saves", d.name, r.name);
            }
        }
    }

    #[test]
    fn a_definition_round_trips_through_the_data_format() {
        for s in super::super::all().iter().filter(|s| !s.subtitle.is_empty()) {
            let def = Def::from_spec(s);
            let again = Def::parse(&def.to_toml()).unwrap_or_else(|e| panic!("{}: {e}", s.name));
            assert_eq!(def, again, "{}", s.name);
        }
    }

    #[test]
    fn mistakes_in_a_definition_are_reported() {
        let ok = "name = \"G\"\ntab = \"T\"\naccent = [1, 2, 3]\nmarkers = [\"/a\"]\nsubtitle = \"S\"\n";
        assert!(Def::parse(ok).is_ok());
        assert!(Def::parse(&format!("{ok}colour = 3\n")).is_err(), "unknown keys are refused");
        assert!(Def::parse(&ok.replace("\"/a\"", "\"a\"")).is_err(), "pointers start with /");
        assert!(Def::parse(&format!("{ok}[[lists]]\ntitle = \"L\"\npath = \"/x\"\nshape = \"array\"\n")).is_err(), "array lists need keys");
    }

    /// Writes the data files for the named Rust screens: `cargo test dump_definitions -- --ignored`
    /// with `DUMP_GAMES="Game one|Game two"`; files go to data/games/<slug>.toml.
    #[test]
    #[ignore]
    fn dump_definitions() {
        let want = std::env::var("DUMP_GAMES").unwrap_or_default();
        for name in want.split('|').filter(|n| !n.is_empty()) {
            let s = super::super::all().iter().find(|s| s.name == name).unwrap_or_else(|| panic!("no screen called {name}"));
            let slug: String = name.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect::<String>().split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/games").join(format!("{slug}.toml"));
            std::fs::write(&path, Def::from_spec(s).to_toml()).unwrap();
            println!("wrote {}", path.display());
        }
    }
}
