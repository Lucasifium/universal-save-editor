//! SQLite database saves (some mobile ports, sims, management games).
//! Tables are shown as lists of rows; edits are applied with real SQL `UPDATE`s on a
//! temporary copy, so the database stays internally consistent.

use crate::tree::{child, int_in, num, Backend, TreeDoc};
use rusqlite::{types::ValueRef, Connection, OpenFlags};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

const MAX_ROWS: usize = 5000;

#[derive(Clone, Debug)]
struct Cell {
    table: String,
    rowid: i64,
    column: String,
    is_int: bool,
}

#[derive(Clone, Debug)]
struct SqliteBackend {
    raw: Vec<u8>,
    cells: Vec<Cell>,
}

/// A temporary copy of the database that's deleted when dropped.
struct Temp(PathBuf);

impl Temp {
    fn new(bytes: &[u8]) -> Result<Self, String> {
        static N: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!("save_editor_{}_{}.sqlite", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        std::fs::write(&p, bytes).map_err(|e| e.to_string())?;
        Ok(Self(p))
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
        for ext in ["-journal", "-wal", "-shm"] {
            let mut p = self.0.as_os_str().to_owned();
            p.push(ext);
            let _ = std::fs::remove_file(PathBuf::from(p));
        }
    }
}

fn quote(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

impl Backend for SqliteBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.raw.clone());
        }
        let tmp = Temp::new(&self.raw)?;
        {
            let conn = Connection::open(&tmp.0).map_err(|e| e.to_string())?;
            let err = |e: rusqlite::Error| e.to_string();
            conn.execute_batch("BEGIN").map_err(err)?;
            for (id, v) in changes {
                let c = &self.cells[*id];
                let sql = format!("UPDATE {} SET {} = ?1 WHERE rowid = ?2", quote(&c.table), quote(&c.column));
                let param: rusqlite::types::Value = match v {
                    Value::Bool(b) => (*b as i64).into(),
                    Value::String(s) => s.clone().into(),
                    Value::Number(n) if c.is_int => int_in(v, i64::MIN as i128, i64::MAX as i128, "This number").map(|x| x as i64)?.into(),
                    Value::Number(n) => n.as_f64().ok_or("bad number")?.into(),
                    _ => return Err("unsupported value".into()),
                };
                conn.execute(&sql, rusqlite::params![param, c.rowid]).map_err(err)?;
            }
            conn.execute_batch("COMMIT").map_err(err)?;
        }
        std::fs::read(&tmp.0).map_err(|e| e.to_string())
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        if self.cells[leaf].is_int && v.is_number() {
            int_in(v, i64::MIN as i128, i64::MAX as i128, "This number")?;
        }
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

/// Primary-key columns and columns covered by a UNIQUE index (lower-case names).
fn key_columns(conn: &Connection, table: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(mut st) = conn.prepare(&format!("PRAGMA table_info({})", quote(table))) {
        if let Ok(rows) = st.query_map([], |r| Ok((r.get::<_, String>(1)?, r.get::<_, i64>(5)?))) {
            out.extend(rows.flatten().filter(|(_, pk)| *pk > 0).map(|(n, _)| n.to_lowercase()));
        }
    }
    let unique_indexes: Vec<String> = conn
        .prepare(&format!("PRAGMA index_list({})", quote(table)))
        .and_then(|mut st| st.query_map([], |r| Ok((r.get::<_, String>(1)?, r.get::<_, i64>(2)?)))?.collect::<Result<Vec<_>, _>>())
        .map(|v| v.into_iter().filter(|(_, unique)| *unique == 1).map(|(n, _)| n).collect())
        .unwrap_or_default();
    for idx in unique_indexes {
        if let Ok(mut st) = conn.prepare(&format!("PRAGMA index_info({})", quote(&idx))) {
            if let Ok(rows) = st.query_map([], |r| r.get::<_, Option<String>>(2)) {
                out.extend(rows.flatten().flatten().map(|n| n.to_lowercase()));
            }
        }
    }
    out
}

/// Columns with `CHECK (col IN (0, 1))` (any spacing/quoting) — really booleans.
fn boolean_columns(sql: &str) -> Vec<String> {
    let flat: String = sql.to_lowercase().chars().filter(|c| !c.is_whitespace() && !"\"`[]".contains(*c)).collect();
    let mut out = Vec::new();
    let mut rest = flat.as_str();
    while let Some(i) = rest.find("check(") {
        rest = &rest[i + 6..];
        if let Some(j) = rest.find("in(0,1)") {
            let col = &rest[..j];
            if !col.is_empty() && col.chars().all(|c| c.is_alphanumeric() || c == '_') {
                out.push(col.to_string());
            }
        }
    }
    out
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let tmp = Temp::new(d)?;
    let conn = Connection::open_with_flags(&tmp.0, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| e.to_string())?;
    let err = |e: rusqlite::Error| e.to_string();
    let tables: Vec<(String, String)> = {
        let mut st = conn
            .prepare("SELECT name, COALESCE(sql, '') FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
            .map_err(err)?;
        let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).map_err(err)?;
        rows.collect::<Result<_, _>>().map_err(err)?
    };
    let mut root = Map::new();
    let mut cells = Vec::new();
    let mut editable = HashMap::new();
    let mut truncated = false;
    for (table, sql) in tables {
        let without_rowid = sql.to_uppercase().contains("WITHOUT ROWID");
        let flag_cols = boolean_columns(&sql);
        let key_cols = key_columns(&conn, &table);
        let query = if without_rowid {
            format!("SELECT * FROM {} LIMIT {}", quote(&table), MAX_ROWS + 1)
        } else {
            // Stable order, so row N is the same row before and after an edit.
            format!("SELECT rowid, * FROM {} ORDER BY rowid LIMIT {}", quote(&table), MAX_ROWS + 1)
        };
        let mut st = conn.prepare(&query).map_err(err)?;
        let names: Vec<String> = st.column_names().iter().map(|s| s.to_string()).collect();
        let first_col = if without_rowid { 0 } else { 1 };
        let mut rows_out = Vec::new();
        let mut rows = st.query([]).map_err(err)?;
        while let Some(row) = rows.next().map_err(err)? {
            if rows_out.len() == MAX_ROWS {
                truncated = true;
                break;
            }
            let i = rows_out.len();
            let rowid: Option<i64> = if without_rowid { None } else { row.get(0).ok() };
            let mut m = Map::new();
            for (c, name) in names.iter().enumerate().skip(first_col) {
                let ptr = child(&format!("{}/{i}", child("", &table)), name);
                let (v, is_int) = match row.get_ref(c).map_err(err)? {
                    ValueRef::Null => (Value::Null, false),
                    // Columns constrained to 0/1 are shown as switches.
                    ValueRef::Integer(x @ (0 | 1)) if flag_cols.contains(&name.to_lowercase()) => (Value::Bool(x == 1), true),
                    ValueRef::Integer(x) => (Value::from(x), true),
                    ValueRef::Real(x) => (num(x), false),
                    ValueRef::Text(t) => (Value::String(String::from_utf8_lossy(t).into_owned()), false),
                    ValueRef::Blob(b) => (Value::String(format!("<{} bytes of data>", b.len())), false),
                };
                let is_blob = matches!(row.get_ref(c).map_err(err)?, ValueRef::Blob(_));
                // IDs (primary keys / unique columns) stay read-only: changing them breaks references.
                let is_key = key_cols.contains(&name.to_lowercase());
                if let (Some(rowid), false, false, false) = (rowid, v.is_null(), is_blob, is_key) {
                    editable.insert(ptr, cells.len());
                    cells.push(Cell { table: table.clone(), rowid, column: name.clone(), is_int });
                }
                m.insert(name.clone(), v);
            }
            rows_out.push(Value::Object(m));
        }
        root.insert(table, Value::Array(rows_out));
    }
    drop(conn);
    let doc = TreeDoc::new(Value::Object(root), editable, "SQLite database", Box::new(SqliteBackend { raw: d.to_vec(), cells }));
    Ok(if truncated { doc.with_note(format!("Large tables: only the first {MAX_ROWS} rows of each table are shown.")) } else { doc })
}
