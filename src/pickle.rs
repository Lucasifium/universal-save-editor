//! Python pickle reader with in-place patching, used for Ren'Py saves.
//!
//! We never rebuild the object graph. Instead the opcode stream is parsed, a
//! display tree (serde_json `Value`) is built, and every editable scalar in that
//! tree remembers which opcode produced it. Saving re-emits the original
//! opcodes, swapping in new bytes only for the scalars that changed — so custom
//! classes, shared references and anything we don't understand stay intact.

use crate::json_view::pointer_escape;
use crate::tree::{Backend, TreeDoc};
use serde_json::{Map, Number, Value};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Clone, Copy, Debug)]
pub struct Op {
    pub code: u8,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone)]
enum PV {
    None,
    Bool(bool, usize),
    Int(i64, usize),
    BigInt,
    Float(f64, usize),
    Str(String, usize),
    Bytes(usize),
    List(Rc<RefCell<Vec<PV>>>),
    Tuple(Rc<Vec<PV>>),
    Dict(Rc<RefCell<Vec<(PV, PV)>>>),
    Set(Rc<RefCell<Vec<PV>>>),
    Global(Rc<str>),
    Obj(Rc<RefCell<PObj>>),
    Mark,
}

struct PObj {
    class: String,
    args: Vec<PV>,
    state: Option<PV>,
    items: Vec<PV>,
    entries: Vec<(PV, PV)>,
}

fn obj(class: String, args: Vec<PV>) -> PV {
    PV::Obj(Rc::new(RefCell::new(PObj {
        class,
        args,
        state: None,
        items: vec![],
        entries: vec![],
    })))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PickleKind {
    RenpySave,
    RenpyPersistent,
    Generic,
}

/// Rewrites a pickle by re-emitting its opcodes, swapping in new bytes for edited scalars.
#[derive(Clone, Debug)]
struct PickleBackend {
    raw: Vec<u8>,
    ops: Vec<Op>,
    /// Ren'Py saves: leaf opcode → opcodes holding that variable's *old* values in the
    /// rollback log. On load Ren'Py rolls back to the last checkpoint using those values,
    /// which would silently undo an edit, so they're updated alongside it.
    links: HashMap<usize, Vec<usize>>,
}

pub fn looks_like_pickle(d: &[u8]) -> bool {
    d.len() > 3 && d[0] == 0x80 && (2..=5).contains(&d[1]) && d.last() == Some(&b'.')
}

/// Parse a pickle into an editable tree document.
pub fn parse(data: &[u8]) -> Result<TreeDoc, String> {
    let (ops, root, shared_ops) = run(data)?;
    let mut conv = Conv {
        editable: HashMap::new(),
        visiting: HashSet::new(),
        root_ptrs: HashMap::new(),
    };
    let (view, kind) = conv.friendly_root(&root);
    let mut links: HashMap<usize, Vec<usize>> = HashMap::new();
    if kind == PickleKind::RenpySave {
        if let PV::Tuple(t) = &root {
            for (full_name, op) in rollback_store_ops(t.get(1)) {
                // A shared (memoised) value may be used elsewhere too — leave it alone.
                if shared_ops.contains(&op) {
                    continue;
                }
                if let Some(leaf) = conv
                    .root_ptrs
                    .get(&full_name)
                    .and_then(|p| conv.editable.get(p))
                {
                    links.entry(*leaf).or_default().push(op);
                }
            }
        }
    }
    let name = match kind {
        PickleKind::RenpySave => "Ren'Py save",
        PickleKind::RenpyPersistent => "Ren'Py persistent data",
        PickleKind::Generic => "Python pickle",
    };
    let backend = PickleBackend {
        raw: data.to_vec(),
        ops,
        links,
    };
    let doc = TreeDoc::new(view, conv.editable, name, Box::new(backend));
    Ok(if kind == PickleKind::Generic {
        doc
    } else {
        doc.with_advanced(vec!["Ren'Py internals".into()])
    })
}

impl Backend for PickleBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.raw.clone());
        }
        let mut patches: HashMap<usize, Vec<u8>> = HashMap::new();
        for (op_idx, cur) in changes {
            let code = self.ops[*op_idx].code;
            patches.insert(*op_idx, encode_scalar(code, cur)?);
            // History values take the variable's new value *and type*.
            for &linked in self.links.get(op_idx).into_iter().flatten() {
                patches.insert(linked, encode_scalar(code, cur)?);
            }
        }
        // Frames (protocol 4+) carry byte lengths that edits would invalidate; they're optional, so drop them.
        let mut out = Vec::with_capacity(self.raw.len() + 64);
        for (i, op) in self.ops.iter().enumerate() {
            if op.code == 0x95 {
                continue;
            }
            match patches.get(&i) {
                Some(bytes) => out.extend_from_slice(bytes),
                None => out.extend_from_slice(&self.raw[op.start..op.end]),
            }
        }
        Ok(out)
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        encode_scalar(self.ops[leaf].code, v).map(|_| ())
    }

    fn linked(&self, leaf: usize) -> usize {
        self.links.get(&leaf).map_or(0, Vec::len)
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

// ------------------------------------------------------------- re-encoding

fn encode_scalar(orig_code: u8, v: &Value) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    match v {
        Value::Bool(b) => {
            if orig_code == b'I' {
                out.extend_from_slice(if *b { b"I01\n" } else { b"I00\n" });
            } else {
                out.push(if *b { 0x88 } else { 0x89 });
            }
        }
        Value::Number(n) if matches!(orig_code, b'F' | b'G') => {
            let f = n.as_f64().ok_or("not a number")?;
            out.push(b'G');
            out.extend_from_slice(&f.to_be_bytes());
        }
        Value::Number(n) => {
            let i = n.as_i64().ok_or("integer out of range")?;
            let is_long = matches!(orig_code, b'L' | 0x8a | 0x8b);
            if !is_long && (0..=0xFF).contains(&i) {
                out.extend_from_slice(&[b'K', i as u8]);
            } else if !is_long && (0..=0xFFFF).contains(&i) {
                out.push(b'M');
                out.extend_from_slice(&(i as u16).to_le_bytes());
            } else if !is_long && i32::try_from(i).is_ok() {
                out.push(b'J');
                out.extend_from_slice(&(i as i32).to_le_bytes());
            } else {
                let mut bytes = i.to_le_bytes().to_vec();
                // Trim to the shortest two's-complement form.
                while bytes.len() > 1 {
                    let last = bytes[bytes.len() - 1];
                    let prev_sign = bytes[bytes.len() - 2] & 0x80;
                    if (last == 0 && prev_sign == 0) || (last == 0xFF && prev_sign != 0) {
                        bytes.pop();
                    } else {
                        break;
                    }
                }
                if i == 0 {
                    bytes.clear();
                }
                out.extend_from_slice(&[0x8a, bytes.len() as u8]);
                out.extend_from_slice(&bytes);
            }
        }
        Value::String(s) => {
            // Keep Python 2 byte-strings as byte-strings, unicode as unicode.
            out.push(if matches!(orig_code, b'S' | b'T' | b'U') {
                b'T'
            } else {
                b'X'
            });
            out.extend_from_slice(&(s.len() as u32).to_le_bytes());
            out.extend_from_slice(s.as_bytes());
        }
        _ => return Err("unsupported value type".into()),
    }
    Ok(out)
}

// ------------------------------------------------------------- the pickle VM

struct Reader<'a> {
    d: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|&e| e <= self.d.len())
            .ok_or("unexpected end of data")?;
        let s = &self.d[self.pos..end];
        self.pos = end;
        Ok(s)
    }
    fn u8(&mut self) -> Result<usize, String> {
        Ok(self.take(1)?[0] as usize)
    }
    fn u16(&mut self) -> Result<usize, String> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()) as usize)
    }
    fn u32(&mut self) -> Result<usize, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()) as usize)
    }
    fn u64(&mut self) -> Result<usize, String> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()) as usize)
    }
    fn line(&mut self) -> Result<String, String> {
        let rest = &self.d[self.pos..];
        let n = rest
            .iter()
            .position(|&b| b == b'\n')
            .ok_or("unterminated line")?;
        self.pos += n + 1;
        Ok(String::from_utf8_lossy(&rest[..n]).into_owned())
    }
}

fn int_from_le(bytes: &[u8]) -> PV {
    if bytes.is_empty() {
        return PV::Int(0, 0);
    }
    if bytes.len() > 8 {
        return PV::BigInt;
    }
    let fill = if bytes[bytes.len() - 1] & 0x80 != 0 {
        0xFF
    } else {
        0
    };
    let mut buf = [fill; 8];
    buf[..bytes.len()].copy_from_slice(bytes);
    PV::Int(i64::from_le_bytes(buf), 0)
}

fn with_op(v: PV, op: usize) -> PV {
    match v {
        PV::Int(i, _) => PV::Int(i, op),
        other => other,
    }
}

fn unquote(s: &str) -> String {
    let t = s.trim();
    let t = t.strip_prefix(['\'', '"']).unwrap_or(t);
    let t = t.strip_suffix(['\'', '"']).unwrap_or(t);
    t.replace("\\'", "'")
        .replace("\\n", "\n")
        .replace("\\\\", "\\")
}

fn unicode_escape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'u') {
            chars.next();
            let hex: String = chars.by_ref().take(4).collect();
            if let Some(ch) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                out.push(ch);
                continue;
            }
            out.push_str("\\u");
            out.push_str(&hex);
        } else {
            out.push(c);
        }
    }
    out
}

fn global_name(v: &PV) -> String {
    match v {
        PV::Global(s) => s.to_string(),
        PV::Obj(o) => o.borrow().class.clone(),
        _ => "<callable>".into(),
    }
}

fn reduce(f: PV, args: PV) -> PV {
    let name = global_name(&f);
    let args: Vec<PV> = match args {
        PV::Tuple(t) => t.as_ref().clone(),
        other => vec![other],
    };
    match name.as_str() {
        "copy_reg._reconstructor" | "copyreg._reconstructor" => {
            let cls = args.first().map(global_name).unwrap_or_default();
            obj(cls, vec![])
        }
        "__builtin__.set" | "builtins.set" | "__builtin__.frozenset" | "builtins.frozenset" => {
            let items = match args.first() {
                Some(PV::List(l)) => l.borrow().clone(),
                Some(PV::Tuple(t)) => t.as_ref().clone(),
                _ => vec![],
            };
            PV::Set(Rc::new(RefCell::new(items)))
        }
        _ => obj(name, args),
    }
}

fn append_to(target: Option<&PV>, items: Vec<PV>) {
    match target {
        Some(PV::List(l)) => l.borrow_mut().extend(items),
        Some(PV::Set(s)) => s.borrow_mut().extend(items),
        Some(PV::Obj(o)) => o.borrow_mut().items.extend(items),
        _ => {}
    }
}

fn set_items(target: Option<&PV>, flat: Vec<PV>) {
    let pairs: Vec<(PV, PV)> = flat
        .chunks(2)
        .filter(|c| c.len() == 2)
        .map(|c| (c[0].clone(), c[1].clone()))
        .collect();
    match target {
        Some(PV::Dict(d)) => d.borrow_mut().extend(pairs),
        Some(PV::Obj(o)) => o.borrow_mut().entries.extend(pairs),
        _ => {}
    }
}

/// Opcode index behind a scalar value, if any.
fn scalar_op(v: &PV) -> Option<usize> {
    match v {
        PV::Bool(_, op) | PV::Int(_, op) | PV::Float(_, op) | PV::Str(_, op) => Some(*op),
        _ => None,
    }
}

/// Look up `key` in a dict-like value (plain dict, revertable dict, or an object's __dict__).
fn dict_entries(v: &PV) -> Vec<(PV, PV)> {
    match v {
        PV::Dict(d) => d.borrow().clone(),
        PV::Obj(o) => {
            let o = o.borrow();
            match &o.state {
                Some(PV::Dict(d)) => d.borrow().clone(),
                Some(PV::Tuple(t)) => match t.first() {
                    Some(PV::Dict(d)) => d.borrow().clone(),
                    _ => vec![],
                },
                _ => o.entries.clone(),
            }
        }
        _ => vec![],
    }
}

fn dict_get(v: &PV, key: &str) -> Option<PV> {
    dict_entries(v)
        .into_iter()
        .find(|(k, _)| matches!(k, PV::Str(s, _) if s == key))
        .map(|(_, v)| v)
}

fn list_items(v: &PV) -> Vec<PV> {
    match v {
        PV::List(l) => l.borrow().clone(),
        PV::Tuple(t) => t.as_ref().clone(),
        PV::Obj(o) => {
            let o = o.borrow();
            if o.items.is_empty() {
                o.args.clone()
            } else {
                o.items.clone()
            }
        }
        _ => vec![],
    }
}

/// Every scalar old-value stored in the Ren'Py rollback log, as ("store.var", opcode).
/// Layout (renpy/rollback.py): RollbackLog.log = [Rollback, ...];
/// Rollback.stores = {"store": {"money": <value before this statement>, ...}, ...}.
fn rollback_store_ops(log: Option<&PV>) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    let Some(entries) = log.and_then(|l| dict_get(l, "log")) else {
        return out;
    };
    for rb in list_items(&entries) {
        let Some(stores) = dict_get(&rb, "stores") else {
            continue;
        };
        for (store_name, changes) in dict_entries(&stores) {
            let PV::Str(store_name, _) = store_name else {
                continue;
            };
            for (var, val) in dict_entries(&changes) {
                if let (PV::Str(var, _), Some(op)) = (var, scalar_op(&val)) {
                    out.push((format!("{store_name}.{var}"), op));
                }
            }
        }
    }
    out
}

fn run(data: &[u8]) -> Result<(Vec<Op>, PV, HashSet<usize>), String> {
    let mut r = Reader { d: data, pos: 0 };
    let mut ops = Vec::new();
    let mut stack: Vec<PV> = Vec::new();
    let mut memo: HashMap<usize, PV> = HashMap::new();
    // Scalars fetched from the memo are referenced in more than one place.
    let mut shared: HashSet<usize> = HashSet::new();
    let mut fetch = |memo: &HashMap<usize, PV>, i: usize| -> Result<PV, String> {
        let v = memo.get(&i).cloned().ok_or("bad memo ref")?;
        if let Some(op) = scalar_op(&v) {
            shared.insert(op);
        }
        Ok(v)
    };

    fn pop(stack: &mut Vec<PV>) -> Result<PV, String> {
        stack.pop().ok_or_else(|| "stack underflow".to_string())
    }
    fn pop_mark(stack: &mut Vec<PV>) -> Result<Vec<PV>, String> {
        let idx = stack
            .iter()
            .rposition(|v| matches!(v, PV::Mark))
            .ok_or("missing MARK")?;
        let items = stack.split_off(idx + 1);
        stack.pop();
        Ok(items)
    }

    loop {
        let start = r.pos;
        let code = r.take(1)?[0];
        let idx = ops.len();
        let mut stop = false;
        match code {
            0x80 => {
                r.take(1)?;
            }
            0x95 => {
                r.take(8)?;
            }
            b'.' => stop = true,
            b'(' => stack.push(PV::Mark),
            b'0' => {
                pop(&mut stack)?;
            }
            b'1' => {
                pop_mark(&mut stack)?;
            }
            b'2' => {
                let top = stack.last().cloned().ok_or("stack underflow")?;
                stack.push(top);
            }
            b'N' => stack.push(PV::None),
            0x88 => stack.push(PV::Bool(true, idx)),
            0x89 => stack.push(PV::Bool(false, idx)),
            b'I' => {
                let l = r.line()?;
                stack.push(match l.as_str() {
                    "00" => PV::Bool(false, idx),
                    "01" => PV::Bool(true, idx),
                    s => s.parse().map(|i| PV::Int(i, idx)).unwrap_or(PV::BigInt),
                });
            }
            b'J' => {
                let v = i32::from_le_bytes(r.take(4)?.try_into().unwrap());
                stack.push(PV::Int(v as i64, idx));
            }
            b'K' => {
                let v = r.u8()?;
                stack.push(PV::Int(v as i64, idx));
            }
            b'M' => {
                let v = r.u16()?;
                stack.push(PV::Int(v as i64, idx));
            }
            b'L' => {
                let l = r.line()?;
                stack.push(
                    l.trim_end_matches('L')
                        .parse()
                        .map(|i| PV::Int(i, idx))
                        .unwrap_or(PV::BigInt),
                );
            }
            0x8a => {
                let n = r.u8()?;
                let b = r.take(n)?;
                stack.push(with_op(int_from_le(b), idx));
            }
            0x8b => {
                let n = r.u32()?;
                let b = r.take(n)?;
                stack.push(with_op(int_from_le(b), idx));
            }
            b'S' => {
                let l = r.line()?;
                stack.push(PV::Str(unquote(&l), idx));
            }
            b'T' => {
                let n = r.u32()?;
                stack.push(PV::Str(
                    String::from_utf8_lossy(r.take(n)?).into_owned(),
                    idx,
                ));
            }
            b'U' => {
                let n = r.u8()?;
                stack.push(PV::Str(
                    String::from_utf8_lossy(r.take(n)?).into_owned(),
                    idx,
                ));
            }
            b'B' => {
                let n = r.u32()?;
                r.take(n)?;
                stack.push(PV::Bytes(n));
            }
            b'C' => {
                let n = r.u8()?;
                r.take(n)?;
                stack.push(PV::Bytes(n));
            }
            0x8e | 0x96 => {
                let n = r.u64()?;
                r.take(n)?;
                stack.push(PV::Bytes(n));
            }
            b'V' => {
                let l = r.line()?;
                stack.push(PV::Str(unicode_escape(&l), idx));
            }
            b'X' => {
                let n = r.u32()?;
                stack.push(PV::Str(
                    String::from_utf8_lossy(r.take(n)?).into_owned(),
                    idx,
                ));
            }
            0x8c => {
                let n = r.u8()?;
                stack.push(PV::Str(
                    String::from_utf8_lossy(r.take(n)?).into_owned(),
                    idx,
                ));
            }
            0x8d => {
                let n = r.u64()?;
                stack.push(PV::Str(
                    String::from_utf8_lossy(r.take(n)?).into_owned(),
                    idx,
                ));
            }
            b'F' => {
                let l = r.line()?;
                stack.push(PV::Float(l.trim().parse().unwrap_or(f64::NAN), idx));
            }
            b'G' => {
                let v = f64::from_be_bytes(r.take(8)?.try_into().unwrap());
                stack.push(PV::Float(v, idx));
            }
            b']' => stack.push(PV::List(Rc::new(RefCell::new(vec![])))),
            b'l' => {
                let items = pop_mark(&mut stack)?;
                stack.push(PV::List(Rc::new(RefCell::new(items))));
            }
            b'a' => {
                let v = pop(&mut stack)?;
                append_to(stack.last(), vec![v]);
            }
            b'e' => {
                let items = pop_mark(&mut stack)?;
                append_to(stack.last(), items);
            }
            b')' => stack.push(PV::Tuple(Rc::new(vec![]))),
            b't' => {
                let items = pop_mark(&mut stack)?;
                stack.push(PV::Tuple(Rc::new(items)));
            }
            0x85..=0x87 => {
                let n = (code - 0x84) as usize;
                if stack.len() < n {
                    return Err("stack underflow".into());
                }
                let items = stack.split_off(stack.len() - n);
                stack.push(PV::Tuple(Rc::new(items)));
            }
            b'}' => stack.push(PV::Dict(Rc::new(RefCell::new(vec![])))),
            b'd' => {
                let flat = pop_mark(&mut stack)?;
                let d = PV::Dict(Rc::new(RefCell::new(vec![])));
                set_items(Some(&d), flat);
                stack.push(d);
            }
            b's' => {
                let v = pop(&mut stack)?;
                let k = pop(&mut stack)?;
                set_items(stack.last(), vec![k, v]);
            }
            b'u' => {
                let flat = pop_mark(&mut stack)?;
                set_items(stack.last(), flat);
            }
            0x8f => stack.push(PV::Set(Rc::new(RefCell::new(vec![])))),
            0x90 => {
                let items = pop_mark(&mut stack)?;
                append_to(stack.last(), items);
            }
            0x91 => {
                let items = pop_mark(&mut stack)?;
                stack.push(PV::Set(Rc::new(RefCell::new(items))));
            }
            b'g' => {
                let i: usize = r.line()?.trim().parse().map_err(|_| "bad GET")?;
                stack.push(fetch(&memo, i)?);
            }
            b'h' => {
                let i = r.u8()?;
                stack.push(fetch(&memo, i)?);
            }
            b'j' => {
                let i = r.u32()?;
                stack.push(fetch(&memo, i)?);
            }
            b'p' => {
                let i: usize = r.line()?.trim().parse().map_err(|_| "bad PUT")?;
                memo.insert(i, stack.last().cloned().ok_or("stack underflow")?);
            }
            b'q' => {
                let i = r.u8()?;
                memo.insert(i, stack.last().cloned().ok_or("stack underflow")?);
            }
            b'r' => {
                let i = r.u32()?;
                memo.insert(i, stack.last().cloned().ok_or("stack underflow")?);
            }
            0x94 => {
                let i = memo.len();
                memo.insert(i, stack.last().cloned().ok_or("stack underflow")?);
            }
            b'c' => {
                let m = r.line()?;
                let n = r.line()?;
                stack.push(PV::Global(format!("{m}.{n}").into()));
            }
            0x93 => {
                let n = pop(&mut stack)?;
                let m = pop(&mut stack)?;
                let s = |v: &PV| {
                    if let PV::Str(s, _) = v {
                        s.clone()
                    } else {
                        "?".into()
                    }
                };
                stack.push(PV::Global(format!("{}.{}", s(&m), s(&n)).into()));
            }
            b'R' => {
                let args = pop(&mut stack)?;
                let f = pop(&mut stack)?;
                stack.push(reduce(f, args));
            }
            0x81 => {
                let args = pop(&mut stack)?;
                let cls = pop(&mut stack)?;
                let args = if let PV::Tuple(t) = args {
                    t.as_ref().clone()
                } else {
                    vec![]
                };
                stack.push(obj(global_name(&cls), args));
            }
            0x92 => {
                pop(&mut stack)?;
                let args = pop(&mut stack)?;
                let cls = pop(&mut stack)?;
                let args = if let PV::Tuple(t) = args {
                    t.as_ref().clone()
                } else {
                    vec![]
                };
                stack.push(obj(global_name(&cls), args));
            }
            b'b' => {
                let state = pop(&mut stack)?;
                match stack.last() {
                    Some(PV::Obj(o)) => o.borrow_mut().state = Some(state),
                    Some(PV::Dict(d)) => {
                        if let PV::Dict(s) = state {
                            let extra = s.borrow().clone();
                            d.borrow_mut().extend(extra);
                        }
                    }
                    _ => {}
                }
            }
            b'i' => {
                let m = r.line()?;
                let n = r.line()?;
                let args = pop_mark(&mut stack)?;
                stack.push(obj(format!("{m}.{n}"), args));
            }
            b'o' => {
                let mut items = pop_mark(&mut stack)?;
                if items.is_empty() {
                    return Err("OBJ without class".into());
                }
                let cls = items.remove(0);
                stack.push(obj(global_name(&cls), items));
            }
            0x82 => {
                r.take(1)?;
                stack.push(PV::Global("<extension>".into()));
            }
            0x83 => {
                r.take(2)?;
                stack.push(PV::Global("<extension>".into()));
            }
            0x84 => {
                r.take(4)?;
                stack.push(PV::Global("<extension>".into()));
            }
            b'P' => {
                r.line()?;
                stack.push(PV::Global("<persistent id>".into()));
            }
            b'Q' => {
                pop(&mut stack)?;
                stack.push(PV::Global("<persistent id>".into()));
            }
            0x97 => stack.push(PV::Bytes(0)),
            0x98 => {}
            other => {
                return Err(format!(
                    "unknown pickle opcode 0x{other:02X} at byte {start}"
                ))
            }
        }
        ops.push(Op {
            code,
            start,
            end: r.pos,
        });
        if stop {
            let root = pop(&mut stack)?;
            drop(fetch);
            return Ok((ops, root, shared));
        }
    }
}

// ------------------------------------------------------------- display tree

const MAX_DEPTH: usize = 48;

struct Conv {
    editable: HashMap<String, usize>,
    visiting: HashSet<usize>,
    /// Full Ren'Py variable name ("store.money") → its pointer in the view.
    root_ptrs: HashMap<String, String>,
}

fn short_class(c: &str) -> &str {
    c.rsplit('.').next().unwrap_or(c)
}

fn key_text(k: &PV) -> String {
    match k {
        PV::Str(s, _) => s.clone(),
        PV::Int(i, _) => i.to_string(),
        PV::Bool(b, _) => {
            if *b {
                "True".into()
            } else {
                "False".into()
            }
        }
        PV::None => "None".into(),
        PV::Float(f, _) => f.to_string(),
        PV::Tuple(t) => format!(
            "({})",
            t.iter().map(key_text).collect::<Vec<_>>().join(", ")
        ),
        PV::Global(g) => g.to_string(),
        PV::Obj(o) => format!("<{}>", short_class(&o.borrow().class)),
        _ => "<key>".into(),
    }
}

fn insert_unique(map: &mut Map<String, Value>, key: String, v: Value) -> String {
    let mut k = key.clone();
    let mut n = 2;
    while map.contains_key(&k) {
        k = format!("{key} ({n})");
        n += 1;
    }
    map.insert(k.clone(), v);
    k
}

impl Conv {
    /// Lay the data out the way a player thinks about it.
    fn friendly_root(&mut self, root: &PV) -> (Value, PickleKind) {
        // Ren'Py save log: (roots, rollback_log) with roots = {"store.gold": 100, ...}
        if let PV::Tuple(t) = root {
            if let Some(PV::Dict(roots)) = t.first() {
                let entries = roots.borrow().clone();
                if entries
                    .iter()
                    .any(|(k, _)| matches!(k, PV::Str(s, _) if s.starts_with("store")))
                {
                    let view = self.split_vars(entries.iter().map(|(k, v)| {
                        let full = key_text(k);
                        let name = full.strip_prefix("store.").unwrap_or(&full).to_string();
                        (name, full, v)
                    }));
                    return (view, PickleKind::RenpySave);
                }
            }
        }
        // Ren'Py persistent file: a Persistent object whose __dict__ holds the fields.
        if let PV::Obj(o) = root {
            let o = o.borrow();
            if o.class.ends_with("Persistent") {
                if let Some(PV::Dict(d)) = &o.state {
                    let entries = d.borrow().clone();
                    let view = self
                        .split_vars(entries.iter().map(|(k, v)| (key_text(k), String::new(), v)));
                    return (view, PickleKind::RenpyPersistent);
                }
            }
        }
        (self.conv(root, "", 0), PickleKind::Generic)
    }

    fn split_vars<'a>(&mut self, items: impl Iterator<Item = (String, String, &'a PV)>) -> Value {
        let mut items: Vec<(String, String, &PV)> = items.collect();
        items.sort_by_key(|(k, _, _)| k.to_lowercase());
        let mut vars = Map::new();
        let mut internal = Map::new();
        for (name, full, v) in items {
            let is_internal =
                name.starts_with('_') || name.starts_with("renpy") || name.contains("._");
            let (group, map) = if is_internal {
                ("Ren'Py internals", &mut internal)
            } else {
                ("Variables", &mut vars)
            };
            let ptr = format!("/{}/{}", pointer_escape(group), pointer_escape(&name));
            if !full.is_empty() {
                self.root_ptrs.insert(full, ptr.clone());
            }
            let val = self.conv(v, &ptr, 1);
            map.insert(name, val);
        }
        let mut out = Map::new();
        out.insert("Variables".into(), Value::Object(vars));
        out.insert("Ren'Py internals".into(), Value::Object(internal));
        Value::Object(out)
    }

    fn conv(&mut self, v: &PV, ptr: &str, depth: usize) -> Value {
        if depth > MAX_DEPTH {
            return Value::String("<nested too deep>".into());
        }
        let rc_id = match v {
            PV::List(l) => Some(Rc::as_ptr(l) as *const () as usize),
            PV::Dict(d) => Some(Rc::as_ptr(d) as *const () as usize),
            PV::Set(s) => Some(Rc::as_ptr(s) as *const () as usize),
            PV::Obj(o) => Some(Rc::as_ptr(o) as *const () as usize),
            _ => None,
        };
        if let Some(id) = rc_id {
            if !self.visiting.insert(id) {
                return Value::String("<circular reference>".into());
            }
        }
        let out = self.conv_inner(v, ptr, depth);
        if let Some(id) = rc_id {
            self.visiting.remove(&id);
        }
        out
    }

    fn list(&mut self, items: &[PV], ptr: &str, depth: usize) -> Value {
        Value::Array(
            items
                .iter()
                .enumerate()
                .map(|(i, c)| self.conv(c, &format!("{ptr}/{i}"), depth + 1))
                .collect(),
        )
    }

    fn dict(
        &mut self,
        entries: &[(PV, PV)],
        ptr: &str,
        depth: usize,
        map: &mut Map<String, Value>,
    ) {
        for (k, v) in entries {
            let key = key_text(k);
            let mut unique = key.clone();
            let mut n = 2;
            while map.contains_key(&unique) {
                unique = format!("{key} ({n})");
                n += 1;
            }
            let val = self.conv(v, &format!("{ptr}/{}", pointer_escape(&unique)), depth + 1);
            insert_unique(map, unique, val);
        }
    }

    fn conv_inner(&mut self, v: &PV, ptr: &str, depth: usize) -> Value {
        match v {
            PV::None | PV::Mark => Value::Null,
            PV::Bool(b, op) => {
                self.editable.insert(ptr.to_string(), *op);
                Value::Bool(*b)
            }
            PV::Int(i, op) => {
                self.editable.insert(ptr.to_string(), *op);
                Value::from(*i)
            }
            PV::BigInt => Value::String("<very large integer>".into()),
            PV::Float(f, op) => match Number::from_f64(*f) {
                Some(n) => {
                    self.editable.insert(ptr.to_string(), *op);
                    Value::Number(n)
                }
                None => Value::String(f.to_string()),
            },
            PV::Str(s, op) => {
                self.editable.insert(ptr.to_string(), *op);
                Value::String(s.clone())
            }
            PV::Bytes(n) => Value::String(format!("<{n} bytes of binary data>")),
            PV::Global(g) => Value::String(format!("<{}>", short_class(g))),
            PV::List(l) => self.list(&l.borrow(), ptr, depth),
            PV::Set(s) => self.list(&s.borrow(), ptr, depth),
            PV::Tuple(t) => self.list(t, ptr, depth),
            PV::Dict(d) => {
                let mut map = Map::new();
                self.dict(&d.borrow(), ptr, depth, &mut map);
                Value::Object(map)
            }
            PV::Obj(o) => {
                let o = o.borrow();
                let class = short_class(&o.class).to_string();
                if o.state.is_none() {
                    if !o.entries.is_empty() && o.items.is_empty() {
                        let mut map = Map::new();
                        self.dict(&o.entries, ptr, depth, &mut map);
                        return Value::Object(map);
                    }
                    if !o.items.is_empty() && o.entries.is_empty() {
                        return self.list(&o.items, ptr, depth);
                    }
                    if o.items.is_empty() && o.entries.is_empty() {
                        if o.args.len() == 1 {
                            return self.conv(&o.args[0], ptr, depth);
                        }
                        if !o.args.is_empty() {
                            return self.list(&o.args, ptr, depth);
                        }
                        if class.contains("Dict") {
                            return Value::Object(Map::new());
                        }
                        if class.contains("List") || class.contains("Set") {
                            return Value::Array(vec![]);
                        }
                        return Value::String(format!("<{class}>"));
                    }
                }
                let mut map = Map::new();
                map.insert("__class__".into(), Value::String(class));
                // State is usually the instance __dict__, or (dict, slots) for slotted classes.
                let mut states: Vec<PV> = Vec::new();
                match &o.state {
                    Some(PV::Tuple(t)) if t.len() == 2 => states.extend(t.iter().cloned()),
                    Some(s) => states.push(s.clone()),
                    None => {}
                }
                for s in &states {
                    match s {
                        PV::Dict(d) => self.dict(&d.borrow(), ptr, depth, &mut map),
                        PV::None => {}
                        other => {
                            let key = insert_unique(&mut map, "state".into(), Value::Null);
                            let val = self.conv(
                                other,
                                &format!("{ptr}/{}", pointer_escape(&key)),
                                depth + 1,
                            );
                            map.insert(key, val);
                        }
                    }
                }
                if !o.entries.is_empty() {
                    let key = insert_unique(&mut map, "entries".into(), Value::Null);
                    let mut inner = Map::new();
                    self.dict(
                        &o.entries,
                        &format!("{ptr}/{}", pointer_escape(&key)),
                        depth + 1,
                        &mut inner,
                    );
                    map.insert(key, Value::Object(inner));
                }
                if !o.items.is_empty() {
                    let key = insert_unique(&mut map, "items".into(), Value::Null);
                    let val = self.list(
                        &o.items,
                        &format!("{ptr}/{}", pointer_escape(&key)),
                        depth + 1,
                    );
                    map.insert(key, val);
                }
                Value::Object(map)
            }
        }
    }
}
