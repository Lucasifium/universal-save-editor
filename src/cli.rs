//! The command line: the same readers and writers as the window, for scripts and batch edits.
//!
//! ```text
//! UniversalSaveEditor info   <save> [--json]
//! UniversalSaveEditor list   <save> [--json]                  every value, by pointer
//! UniversalSaveEditor get    <save> <pointer>...
//! UniversalSaveEditor set    <save> <pointer>=<json>... [-o <out>]
//! UniversalSaveEditor decode <save> [-o <out>]                the unpacked data
//! UniversalSaveEditor encode <save> <edited data> [-o <out>]  pack edited data like <save>
//! UniversalSaveEditor patch  <save> [<game|file> [<n>[:<k>]...]]  console cheat patches (Apollo)
//! UniversalSaveEditor games  [--json]                         the games the editor knows
//! UniversalSaveEditor backup  <save> [--name <n>]             keep a copy of the save as it is now
//! UniversalSaveEditor backups <save> [--json]                 the kept copies, newest first
//! UniversalSaveEditor restore <save> <n>                      put copy <n> back
//! UniversalSaveEditor presets [--json]                        the presets that can be applied
//! UniversalSaveEditor apply   <save> <preset> [--dry-run]     make a preset's changes
//! ```
//!
//! Writing is as careful as the window: the new file must decode to exactly what was asked
//! for before anything is written, the old file is backed up first (File › Restore previous
//! version lists the copies), and the write is atomic. `--password <p>` decrypts like
//! Tools › Decrypt with password.

use crate::formats::{self, Content, Options};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const COMMANDS: [&str; 16] = ["info", "list", "get", "set", "decode", "encode", "checksum", "steamid", "patch", "games", "backup", "backups", "restore", "presets", "apply", "help"];

/// Runs a command if `args` (without the program name) start with one; `None` means "open the
/// window", with the first argument as a file to open.
pub fn run(args: &[String]) -> Option<i32> {
    let first = args.first()?.as_str();
    let cmd = first.trim_start_matches('-');
    if !COMMANDS.contains(&cmd) && !matches!(first, "-h" | "--help" | "--version" | "-V") {
        return None;
    }
    if matches!(first, "--version" | "-V") {
        println!("{} {}", crate::platform::DISPLAY_NAME, env!("CARGO_PKG_VERSION"));
        return Some(0);
    }
    let code = match go(cmd, &args[1..]) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {e}");
            if e.starts_with("usage") {
                2
            } else {
                1
            }
        }
    };
    Some(code)
}

struct Args {
    pos: Vec<String>,
    json: bool,
    out: Option<PathBuf>,
    password: Option<String>,
    entry: Option<String>,
    no_backup: bool,
    /// --be / --le: byte order for cheat patches.
    big_endian: Option<bool>,
    /// --name: what to call a backup kept with `backup`.
    name: Option<String>,
    /// --dry-run: show what `apply` would change and write nothing.
    dry_run: bool,
}

fn parse(args: &[String]) -> Result<Args, String> {
    let mut a = Args { pos: vec![], json: false, out: None, password: None, entry: None, no_backup: false, big_endian: None, name: None, dry_run: false };
    let mut it = args.iter();
    while let Some(x) = it.next() {
        match x.as_str() {
            "--json" => a.json = true,
            "--no-backup" => a.no_backup = true,
            "--dry-run" => a.dry_run = true,
            "--name" => a.name = Some(it.next().ok_or("usage: --name needs a name for the backup")?.clone()),
            "--be" => a.big_endian = Some(true),
            "--le" => a.big_endian = Some(false),
            "-o" | "--out" => a.out = Some(PathBuf::from(it.next().ok_or("usage: -o needs a file")?)),
            "--password" => a.password = Some(it.next().ok_or("usage: --password needs a value")?.clone()),
            "--entry" => a.entry = Some(it.next().ok_or("usage: --entry needs a name (see info)")?.clone()),
            _ => a.pos.push(x.clone()),
        }
    }
    Ok(a)
}

fn help() {
    println!(
        "{} {} \u{2014} command line\n\n\
         Usage:\n  \
         UniversalSaveEditor <save>                         open the save in the window\n  \
         UniversalSaveEditor info   <save> [--json]         what the file is and which screen opens it\n  \
         UniversalSaveEditor list   <save> [--json]         every value, by pointer\n  \
         UniversalSaveEditor get    <save> <pointer>...     print values\n  \
         UniversalSaveEditor set    <save> <pointer>=<json>... [-o <out>] [--no-backup]\n  \
         UniversalSaveEditor decode <save> [-o <out>]       the unpacked data (JSON, text or bytes)\n  \
         UniversalSaveEditor encode <save> <data> [-o <out>]  pack edited data the way <save> is packed\n  \
         UniversalSaveEditor checksum <save> [<n>|off]      find a checksum; <n> keeps that one fixed on every save\n  \
         UniversalSaveEditor steamid <save> [<old> <new>]   list the Steam IDs in a save, or change one to another account\n  \
         UniversalSaveEditor patch  <save> [<game> [<n>...]]  console cheat patches: list the patch files for the save,\n                                                    a patch file's codes, or apply codes by number (<n>:<k> picks\n                                                    option k of a code with options; --be/--le sets the byte order)\n  \
         UniversalSaveEditor games  [--json]                the games the editor knows\n  \
         UniversalSaveEditor backup  <save> [--name <n>]    keep a copy of the save as it is now (a name keeps it for good)\n  \
         UniversalSaveEditor backups <save> [--json]        the kept copies, newest first, numbered\n  \
         UniversalSaveEditor restore <save> <n>             put copy <n> back (the save as it is now is backed up first)\n  \
         UniversalSaveEditor presets [--json]               the presets you can apply: your own and the ready-made ones\n  \
         UniversalSaveEditor apply   <save> <preset> [--dry-run] [-o <out>]  make a preset's changes (a name or a .json file)\n\n\
         Options: --password <p> (encrypted saves), --entry <name> (one save in an archive or\n\
         memory card, as listed by info), --json (machine-readable output).\n\
         Pointers are JSON pointers as shown by `list` (for example /money or /party/0/hp).\n\
         Values are JSON: 999, 2.5, true, \"text\".\n\
         Writing checks the new file reads back exactly, backs up the old one, and replaces it atomically.",
        crate::platform::DISPLAY_NAME,
        env!("CARGO_PKG_VERSION")
    );
}

/// `patch`: <game> is a title ID, a name to search for, or a .savepatch file.
fn patch(a: &Args) -> Result<(), String> {
    use crate::apollo::{self, db, Kind};
    let f = a.pos.first().ok_or("usage: patch <save> [<game|file.savepatch> [<n>[:<k>]...]]")?;
    let (path, opts, mut d) = open(f, a)?;
    let Content::Binary(data) = &d.content else {
        return Err(format!("{f}: cheat patches work on a console save's raw data, and this file opens as structured data"));
    };
    let inner = d.layers.iter().rev().find_map(|l| match l {
        formats::Layer::ZipEntry { name, .. } | formats::Layer::Copeck { name, .. } => Some(name.clone()),
        formats::Layer::Ps2Card { file, .. } => Some(file.name.clone()),
        formats::Layer::PsSave { name, .. } => Some(name.clone()),
        _ => None,
    });
    let layer = d.layers.iter().find_map(|l| if let formats::Layer::Apollo { var, .. } = l { Some(var.clone()) } else { None });
    let full = format!("{}{}", path.to_string_lossy().replace('\\', "/"), inner.map(|n| format!("/{n}")).unwrap_or_default());
    let mut file = full.rsplit('/').next().unwrap_or_default().to_string();
    if let Some(Some(v)) = &layer {
        file = v.clone();
    }
    let Some(game) = a.pos.get(1) else {
        let ids = db::ids_in(&full);
        if ids.is_empty() {
            println!("no title ID in the save's name; name the game: patch <save> <title ID or name>");
        }
        for e in ids.iter().flat_map(|id| db::find(id)) {
            println!("{}  {}  ({})", e.id, e.title, e.platform);
        }
        return Ok(());
    };
    let (label, codes, mut be) = if game.to_ascii_lowercase().ends_with(".savepatch") {
        let raw = std::fs::read(game).map_err(|e| format!("{game}: {e}"))?;
        let (text, latin1) = match String::from_utf8(raw) {
            Ok(t) => (t, false),
            Err(e) => (e.into_bytes().iter().map(|&b| b as char).collect(), true),
        };
        let (id, title) = apollo::header(&text);
        let be = db::ps3_id(if id.is_empty() { game } else { &id });
        (title, apollo::parse(&text, latin1), be)
    } else {
        let mut hits = db::find(game);
        if hits.is_empty() {
            hits = db::search(game, 20);
        }
        match hits[..] {
            [] => return Err(format!("no patch file for \"{game}\"")),
            [e] => (format!("{} ({})", e.title, e.platform), e.codes(), e.big_endian()),
            _ => {
                println!("several patch files match; use a title ID:");
                hits.iter().for_each(|e| println!("{}  {}  ({})", e.id, e.title, e.platform));
                return Ok(());
            }
        }
    };
    be = a.big_endian.unwrap_or(be);
    let picked = &a.pos[2..];
    if picked.is_empty() {
        println!("{label}  [{}]", if be { "big-endian" } else { "little-endian" });
        for (i, c) in codes.iter().enumerate() {
            if c.empty() {
                println!("  -- {} --", c.name);
                continue;
            }
            let mut notes = vec![];
            if c.required {
                notes.push("always applied".to_string());
            }
            if c.kind == Kind::Python {
                notes.push("Python, not supported".into());
            }
            if !apollo::targets(c, &file) {
                notes.push(format!("for {}", c.file));
            }
            let notes = if notes.is_empty() { String::new() } else { format!("  ({})", notes.join(", ")) };
            println!("{:>4}  {}{}{notes}", i + 1, if c.child { "  " } else { "" }, c.name);
            for ch in &c.choices {
                let opts: Vec<String> = ch.values.iter().enumerate().map(|(k, (_, n))| format!("{}={n}", k + 1)).collect();
                println!("        options: {}", opts.join(", "));
            }
        }
        return Ok(());
    }
    // Which codes, and the option picked for each of their {TAGS} (1-based on the command line).
    let mut want: Vec<(usize, Vec<usize>)> = vec![];
    for p in picked.iter().flat_map(|p| p.split(',')).filter(|p| !p.is_empty()) {
        let mut it = p.split(':');
        let n: usize = it.next().and_then(|n| n.parse().ok()).filter(|&n| n >= 1 && n <= codes.len()).ok_or(format!("{p}: not a code number from the list"))?;
        let ks: Vec<usize> = it.map(|k| k.parse::<usize>().ok().filter(|&k| k >= 1).map(|k| k - 1).ok_or(format!("{p}: options are numbered from 1"))).collect::<Result<_, _>>()?;
        let c = &codes[n - 1];
        if c.empty() {
            return Err(format!("{n} is a heading, not a code"));
        }
        for (i, k) in ks.iter().enumerate() {
            if c.choices.get(i).is_none_or(|ch| *k >= ch.values.len()) {
                return Err(format!("{p}: code {n} has no option {}", k + 1));
            }
        }
        want.push((n - 1, ks));
    }
    let mut out = data.clone();
    let mut session = apollo::Session::new(be);
    if let Some(Some(v)) = &layer {
        session.seed(v, out.clone());
    }
    let mut done = 0;
    for (i, c) in codes.iter().enumerate() {
        let picks = want.iter().find(|(n, _)| *n == i).map(|(_, k)| k.clone());
        // Required codes unpack and pack the save: when it opened unpacked, they already ran.
        if (picks.is_none() && !(c.required && layer.is_none() && apollo::targets(c, &file))) || c.empty() {
            continue;
        }
        session.apply(&mut out, c, picks.as_deref().unwrap_or(&[])).map_err(|e| format!("{}: {e}; nothing was written", c.name))?;
        done += usize::from(picks.is_some());
    }
    if let Some(Some(v)) = &layer {
        out = session.unpacked(v).unwrap_or(&out).to_vec();
    }
    if out == *data {
        println!("the codes didn't change anything");
        return Ok(());
    }
    d.content = Content::Binary(out);
    let bytes = formats::encode(&d.layers, &d.content)?;
    let check = formats::decode_with(&bytes, &opts);
    if !formats::same_layers(&check.layers, &d.layers) || !formats::verified(&check.content, &d.content) {
        return Err("the patched save didn't read back identically; nothing was written".into());
    }
    write(&path, a.out.as_deref(), &bytes, a.no_backup)?;
    println!("applied {done} code{}", if done == 1 { "" } else { "s" });
    Ok(())
}

fn open(path: &str, a: &Args) -> Result<(PathBuf, Options, formats::Decoded), String> {
    let p = PathBuf::from(path);
    let raw = std::fs::read(&p).map_err(|e| format!("{path}: {e}"))?;
    let mut opts = Options::for_path(&p);
    opts.password = a.password.clone();
    opts.zip_entry = a.entry.clone();
    let d = formats::decode_with(&raw, &opts);
    Ok((p, opts, d))
}

fn root(c: &Content) -> Option<&Value> {
    match c {
        Content::Json { value, .. } => Some(value),
        Content::Tree(t) => Some(&t.view),
        _ => None,
    }
}

fn screen(c: &Content) -> Option<String> {
    let g = match c {
        Content::Tree(t) => crate::game_ui::detect(&t.kind_name, &t.view),
        Content::Json { value, .. } => crate::game_ui::detect("JSON", value),
        _ => None,
    }?;
    Some(format!("{g:?}"))
}

fn flatten(v: &Value, ptr: &str, out: &mut Vec<(String, Value)>) {
    match v {
        Value::Object(m) => m.iter().for_each(|(k, c)| flatten(c, &crate::tree::child(ptr, k), out)),
        Value::Array(a) => a.iter().enumerate().for_each(|(i, c)| flatten(c, &format!("{ptr}/{i}"), out)),
        other => out.push((ptr.to_string(), other.clone())),
    }
}

fn go(cmd: &str, args: &[String]) -> Result<(), String> {
    let a = parse(args)?;
    match cmd {
        "help" | "h" => help(),
        "checksum" => {
            let f = a.pos.first().ok_or("usage: checksum <save> [<n>|off]")?;
            let (path, _, d) = open(f, &a)?;
            let Content::Binary(b) = &d.content else {
                return Err(format!("{f}: the editor already reads this file ({}); its checksums are handled", formats::describe(&d.layers, &d.content)));
            };
            let others: Vec<Vec<u8>> = crate::backups::list(&path).into_iter().filter(|v| v.size as usize == b.len()).take(8).filter_map(|v| std::fs::read(&v.path).ok()).collect();
            let hits = crate::fmt::sumfind::find(b, &others);
            match a.pos.get(1).map(String::as_str) {
                Some("off") => {
                    crate::fmt::sumfind::remember(&path, None)?;
                    println!("{f}: no checksum is fixed on save any more");
                }
                Some(n) => {
                    let h = n.parse::<usize>().ok().and_then(|i| hits.get(i.wrapping_sub(1))).ok_or("usage: checksum <save> <n>, with n from the list")?;
                    crate::fmt::sumfind::remember(&path, Some(*h))?;
                    println!("{f}: {} is now written again on every save", h.describe());
                }
                None if a.json => println!("{}", serde_json::to_string_pretty(&hits.iter().map(|h| h.describe()).collect::<Vec<_>>()).unwrap()),
                None if hits.is_empty() => println!("{f}: no known checksum at the start or end of the file"),
                None => hits.iter().enumerate().for_each(|(i, h)| println!("{:>3}  {}", i + 1, h.describe())),
            }
        }
        "steamid" => {
            let f = a.pos.first().ok_or("usage: steamid <save> [<old> <new>]")?;
            let (path, opts, mut d) = open(f, &a)?;
            let found: Vec<u64> = match &d.content {
                Content::Binary(b) => crate::steamid::in_bytes(b).into_iter().map(|(id, _)| id).collect(),
                c => root(c).map(|v| crate::steamid::in_values(v).into_iter().map(|(id, _)| id).collect()).unwrap_or_default(),
            };
            let (Some(old), Some(new)) = (a.pos.get(1), a.pos.get(2)) else {
                if found.is_empty() {
                    println!("{f}: no Steam ID found in the save's data");
                }
                found.iter().for_each(|id| println!("{id}  (account {})", id - crate::steamid::BASE));
                return Ok(());
            };
            let (old, new) = (crate::steamid::parse(old).ok_or("<old> isn't a Steam ID")?, crate::steamid::parse(new).ok_or("<new> isn't a Steam ID")?);
            if !found.contains(&old) {
                return Err(format!("{f}: {old} isn't in this save"));
            }
            let n = match &mut d.content {
                Content::Binary(b) => {
                    let at = crate::steamid::in_bytes(b).into_iter().find(|(id, _)| *id == old).map(|(_, a)| a).unwrap_or_default();
                    at.iter().for_each(|&o| b[o..o + 8].copy_from_slice(&new.to_le_bytes()));
                    at.len()
                }
                c => {
                    let v = root(c).cloned().unwrap_or_default();
                    let ptrs = crate::steamid::in_values(&v).into_iter().find(|(id, _)| *id == old).map(|(_, p)| p).unwrap_or_default();
                    for p in &ptrs {
                        let nv = crate::steamid::replaced(v.pointer(p).unwrap_or(&Value::Null), new);
                        set_in(c, p, nv)?;
                    }
                    ptrs.len()
                }
            };
            let bytes = formats::encode(&d.layers, &d.content)?;
            let check = formats::decode_with(&bytes, &opts);
            if !formats::same_layers(&check.layers, &d.layers) || !formats::verified(&check.content, &d.content) {
                return Err("the changed save didn't read back identically; nothing was written".into());
            }
            write(&path, a.out.as_deref(), &bytes, a.no_backup)?;
            println!("changed {n} place{}", if n == 1 { "" } else { "s" });
        }
        "patch" => patch(&a)?,
        "games" => {
            if a.json {
                let list: Vec<Value> = crate::games::GAMES
                    .iter()
                    .map(|g| json!({"name": g.name, "support": g.support, "folder": g.folder, "files": g.files}))
                    .collect();
                println!("{}", serde_json::to_string_pretty(&list).unwrap());
            } else {
                for g in crate::games::GAMES {
                    println!("{}\n    {}\n    {}  ({})", g.name, g.support, g.folder, g.files);
                }
            }
        }
        "info" => {
            let f = a.pos.first().ok_or("usage: info <save>")?;
            let (_, _, d) = open(f, &a)?;
            let layers: Vec<String> = d.layers.iter().map(|l| l.name()).collect();
            let size = d.content.to_bytes().map(|b| b.len()).unwrap_or(0);
            let values = root(&d.content).map(|r| {
                let mut v = vec![];
                flatten(r, "", &mut v);
                v.len()
            });
            if a.json {
                let out = json!({
                    "file": f,
                    "layers": layers,
                    "content": d.content.kind(),
                    "screen": screen(&d.content),
                    "values": values,
                    "decoded_bytes": size,
                    "hint": d.hint,
                    "entries": d.zip_entries,
                });
                println!("{}", serde_json::to_string_pretty(&out).unwrap());
            } else {
                println!("File:     {f}");
                println!("Format:   {}", formats::describe(&d.layers, &d.content));
                if let Some(s) = screen(&d.content) {
                    println!("Screen:   {s}");
                }
                if let Some(n) = values {
                    println!("Values:   {n}");
                }
                println!("Decoded:  {size} bytes");
                if let Some(h) = &d.hint {
                    println!("Note:     {h}");
                }
                if !d.zip_entries.is_empty() {
                    println!("Entries (pick one with --entry):");
                    d.zip_entries.iter().for_each(|e| println!("  {e}"));
                }
            }
        }
        "list" => {
            let f = a.pos.first().ok_or("usage: list <save>")?;
            let (_, _, d) = open(f, &a)?;
            let r = root(&d.content).ok_or_else(|| format!("{f} has no named values ({}); use decode, or the window's Value search", d.content.kind()))?;
            let mut v = vec![];
            flatten(r, "", &mut v);
            if a.json {
                let m: serde_json::Map<String, Value> = v.into_iter().collect();
                println!("{}", serde_json::to_string_pretty(&m).unwrap());
            } else {
                for (p, x) in v {
                    println!("{p} = {x}");
                }
            }
        }
        "get" => {
            let (f, ptrs) = a.pos.split_first().ok_or("usage: get <save> <pointer>...")?;
            if ptrs.is_empty() {
                return Err("usage: get <save> <pointer>...".into());
            }
            let (_, _, d) = open(f, &a)?;
            let r = root(&d.content).ok_or("this save has no named values")?;
            for p in ptrs {
                let v = r.pointer(p).ok_or_else(|| format!("{p}: not in this save"))?;
                if ptrs.len() == 1 {
                    println!("{v}");
                } else {
                    println!("{p} = {v}");
                }
            }
        }
        "set" => {
            let (f, edits) = a.pos.split_first().ok_or("usage: set <save> <pointer>=<json>...")?;
            if edits.is_empty() {
                return Err("usage: set <save> <pointer>=<json>...".into());
            }
            let (path, opts, mut d) = open(f, &a)?;
            for e in edits {
                let (ptr, text) = e.split_once('=').ok_or_else(|| format!("usage: {e}: write pointer=value"))?;
                let old = root(&d.content).and_then(|r| r.pointer(ptr)).cloned().ok_or_else(|| format!("{ptr}: not in this save"))?;
                // Bare words are taken as text, so `/name=Ann` works without quotes; but only
                // for a value that is text. A typo (`/gold=1,000`, `/gold=99O`) must not turn a
                // number into text the game can't read.
                let v: Value = match serde_json::from_str::<Value>(text) {
                    // A name such as 123 or true stays text where the save holds text.
                    Ok(v) if old.is_string() && !v.is_string() => Value::String(text.to_string()),
                    Ok(v) => v,
                    Err(_) if old.is_string() => Value::String(text.to_string()),
                    Err(_) => {
                        let kind = match &old {
                            Value::Number(_) => "a number",
                            Value::Bool(_) => "true or false",
                            Value::Null => "null",
                            Value::Array(_) => "a list",
                            _ => "a group of values",
                        };
                        let shown = old.to_string().chars().take(40).collect::<String>();
                        return Err(format!("{ptr} holds {kind} ({shown}), and {text} isn't one; write numbers like 1000 or 2.5, or put text in quotes ({ptr}='\"{text}\"'). Nothing was written"));
                    }
                };
                match &mut d.content {
                    Content::Tree(t) => {
                        t.set(ptr, v.clone()).map_err(|e| format!("{ptr}: {e}"))?.ok_or_else(|| format!("{ptr}: can't be edited"))?;
                    }
                    Content::Json { value, .. } => *value.pointer_mut(ptr).unwrap() = v.clone(),
                    _ => unreachable!(),
                }
                let now = root(&d.content).and_then(|r| r.pointer(ptr)).cloned().unwrap_or(Value::Null);
                eprintln!("{ptr}: {old} \u{203A} {now}");
            }
            write_edited(&path, &opts, &d, &a)?;
        }
        "backup" => {
            let f = a.pos.first().ok_or("usage: backup <save> [--name <n>]")?;
            let path = PathBuf::from(f);
            if !path.is_file() {
                return Err(format!("{f}: no such file"));
            }
            match &a.name {
                Some(n) => {
                    let kept = crate::backups::take_named(&path, n)?;
                    // The name as it is kept: characters a file name can't hold are replaced.
                    let n = crate::backups::list(&path).into_iter().find(|v| v.path == kept).and_then(|v| v.name).unwrap_or_else(|| n.clone());
                    println!("kept {} as \u{201C}{n}\u{201D}: {}", f, kept.display());
                }
                None => match crate::backups::take(&path)? {
                    Some(b) => println!("backed up {} as {}", f, b.display()),
                    None => println!("{f}: the newest backup already holds exactly this file"),
                },
            }
        }
        "backups" => {
            let f = a.pos.first().ok_or("usage: backups <save> [--json]")?;
            let list = crate::backups::list(Path::new(f));
            if a.json {
                let v: Vec<Value> = list.iter().enumerate().map(|(i, v)| json!({"n": i + 1, "when": v.when, "name": v.name, "size": v.size, "file": v.path.display().to_string()})).collect();
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            } else if list.is_empty() {
                println!("{f}: no backups yet");
            } else {
                for (i, v) in list.iter().enumerate() {
                    let name = v.name.as_deref().map(|n| format!("  \u{201C}{n}\u{201D}")).unwrap_or_default();
                    println!("{:>3}  {}  {} bytes{name}", i + 1, crate::backups::describe(v.when), v.size);
                }
            }
        }
        "restore" => {
            let (Some(f), Some(n)) = (a.pos.first(), a.pos.get(1)) else { return Err("usage: restore <save> <n>, with n from `backups`".into()) };
            let path = PathBuf::from(f);
            let list = crate::backups::list(&path);
            let v = n.parse::<usize>().ok().and_then(|i| list.get(i.wrapping_sub(1))).ok_or_else(|| format!("usage: restore <save> <n>: there is no backup {n} (see `backups {f}`)"))?;
            if let Some(game) = crate::running::game_for(&path) {
                return Err(format!("{game} looks like it's running; quit it first, or it may overwrite the restored save. Nothing was restored"));
            }
            let before = crate::backups::Kept::read(&path).map_err(|e| format!("{f}: {e}"))?;
            crate::backups::restore(&path, &v.path).map_err(|e| format!("couldn't restore: {e}. The save on disk wasn't changed"))?;
            let bytes = std::fs::read(&path).map_err(|e| format!("{f}: {e}"))?;
            crate::backups::write_companions(&before, formats::companion_files(&path, &bytes), true).map_err(|e| format!("{}: {}; the save itself was restored", e.file.display(), e.error))?;
            println!("restored {f} from backup {n} ({}); the save it replaced was backed up too", crate::backups::describe(v.when));
        }
        "presets" => {
            let all = all_presets();
            if a.json {
                let v: Vec<Value> = all.iter().map(|(file, p)| json!({"name": p.name, "changes": p.len(), "built_in": p.built_in, "file": file.as_ref().map(|f| f.display().to_string())})).collect();
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            } else if all.is_empty() {
                println!("no presets");
            } else {
                for (file, p) in &all {
                    let kind = if p.built_in { "built in".to_string() } else { file.as_ref().map(|f| f.display().to_string()).unwrap_or_default() };
                    println!("{}  ({} change{})  {kind}", p.name, p.len(), if p.len() == 1 { "" } else { "s" });
                }
            }
        }
        "apply" => {
            let (Some(f), Some(which)) = (a.pos.first(), a.pos.get(1)) else { return Err("usage: apply <save> <preset name or .json file>".into()) };
            let preset = find_preset(which)?;
            let (path, opts, mut d) = open(f, &a)?;
            let (mut changed, mut missing) = (0usize, 0usize);
            match &mut d.content {
                Content::Binary(b) => {
                    for c in &preset.bytes {
                        if c.offset + c.bytes.len() > b.len() {
                            missing += 1;
                            continue;
                        }
                        let old = b[c.offset..c.offset + c.bytes.len()].to_vec();
                        b[c.offset..c.offset + c.bytes.len()].copy_from_slice(&c.bytes);
                        if old != c.bytes {
                            changed += 1;
                            eprintln!("{} (0x{:X}): {:02X?} \u{203A} {:02X?}", c.name, c.offset, old, c.bytes);
                        }
                    }
                    missing += preset.values.len();
                }
                c => {
                    let (todo, gone) = match root(c) {
                        Some(r) => crate::presets::resolve(&preset.values, r),
                        None => (vec![], preset.values.len()),
                    };
                    missing += gone + preset.bytes.len();
                    for ch in todo {
                        let old = root(c).and_then(|r| r.pointer(&ch.path)).cloned().unwrap_or(Value::Null);
                        set_in(c, &ch.path, ch.value.clone())?;
                        let now = root(c).and_then(|r| r.pointer(&ch.path)).cloned().unwrap_or(Value::Null);
                        if now != old {
                            changed += 1;
                            eprintln!("{} {}: {old} \u{203A} {now}", ch.name, ch.path);
                        }
                    }
                }
            }
            eprintln!("{}: {changed} change{} made{}", preset.name, if changed == 1 { "" } else { "s" }, if missing > 0 { format!(", {missing} not in this save and left out") } else { String::new() });
            if changed == 0 {
                println!("{f}: nothing to change; the file was not written");
            } else if a.dry_run {
                println!("{f}: dry run, nothing was written");
            } else {
                write_edited(&path, &opts, &d, &a)?;
            }
        }
        "decode" => {
            let f = a.pos.first().ok_or("usage: decode <save> [-o <out>]")?;
            let (_, _, d) = open(f, &a)?;
            let bytes = d.content.to_bytes()?;
            match &a.out {
                Some(o) => {
                    // `-o` may name a save (even this one): keep what is there first.
                    if !a.no_backup {
                        if let Some(b) = crate::backups::take(o)? {
                            eprintln!("backed up the old file as {}", b.display());
                        }
                    }
                    crate::backups::write_verified(o, &bytes, true).map_err(|e| format!("{}: {e}", o.display()))?
                }
                None => {
                    use std::io::Write;
                    std::io::stdout().write_all(&bytes).map_err(|e| e.to_string())?;
                }
            }
        }
        "encode" => {
            let [f, data] = &a.pos[..] else { return Err("usage: encode <save> <edited data> [-o <out>]".into()) };
            let (path, opts, d) = open(f, &a)?;
            let payload = std::fs::read(data).map_err(|e| format!("{data}: {e}"))?;
            let bytes = formats::encode(&d.layers, &Content::Binary(payload.clone()))?;
            let check = formats::decode_with(&bytes, &opts);
            let same = check.content.to_bytes()? == payload || {
                // JSON may come back with other spacing: compare the values.
                let want = formats::decode_with(&payload, &Options::default());
                want.layers.is_empty() && formats::same_data(&check.content, &want.content)
            };
            if !formats::same_layers(&check.layers, &d.layers) || !same {
                return Err("the packed file didn't unpack to the same data; nothing was written".into());
            }
            write(&path, a.out.as_deref(), &bytes, a.no_backup)?;
        }
        _ => return Err(format!("usage: unknown command {cmd}")),
    }
    Ok(())
}

/// Packs the edited data, checks the new file reads back exactly as intended and only then writes it.
fn write_edited(path: &Path, opts: &Options, d: &formats::Decoded, a: &Args) -> Result<(), String> {
    let bytes = formats::encode(&d.layers, &d.content)?;
    let check = formats::decode_with(&bytes, opts);
    if !formats::same_layers(&check.layers, &d.layers) || !formats::verified(&check.content, &d.content) {
        return Err("the new file didn't read back identically; nothing was written".into());
    }
    write(path, a.out.as_deref(), &bytes, a.no_backup)
}

/// Every preset the editor can apply: the player's files first, then the ready-made ones.
fn all_presets() -> Vec<(Option<PathBuf>, crate::presets::Preset)> {
    let mut v: Vec<_> = crate::presets::list().into_iter().map(|(f, p)| (Some(f), p)).collect();
    v.extend(crate::presets::built_in().into_iter().map(|p| (None, p)));
    v
}

/// A preset named on the command line: a .json file, or the name (or file name) of a kept or
/// ready-made one, ignoring case.
fn find_preset(which: &str) -> Result<crate::presets::Preset, String> {
    let as_file = Path::new(which);
    if as_file.is_file() {
        let text = std::fs::read_to_string(as_file).map_err(|e| format!("{which}: {e}"))?;
        return crate::presets::Preset::from_json(&text).map_err(|e| format!("{which} isn't a preset: {e}"));
    }
    let want = which.trim().to_lowercase();
    let all = all_presets();
    all.iter()
        .find(|(f, p)| {
            p.name.to_lowercase() == want || f.as_ref().and_then(|f| f.file_stem()).is_some_and(|s| s.to_string_lossy().to_lowercase() == want)
        })
        .map(|(_, p)| p.clone())
        .ok_or_else(|| format!("no preset called \u{201C}{which}\u{201D} (see `presets`)"))
}

/// Sets the value at `ptr` in JSON or Tree content.
fn set_in(c: &mut Content, ptr: &str, v: Value) -> Result<(), String> {
    match c {
        Content::Tree(t) => {
            t.set(ptr, v).map_err(|e| format!("{ptr}: {e}"))?.ok_or_else(|| format!("{ptr}: can't be edited"))?;
        }
        Content::Json { value, .. } => *value.pointer_mut(ptr).ok_or_else(|| format!("{ptr}: not in this save"))? = v,
        _ => return Err(format!("{ptr}: this save has no values by name")),
    }
    Ok(())
}

/// Writes `bytes` to `out` (or over `path`), backing up whatever is replaced, atomically.
fn write(path: &Path, out: Option<&Path>, bytes: &[u8], no_backup: bool) -> Result<(), String> {
    let target = out.unwrap_or(path);
    if target.exists() && !no_backup {
        if let Some(b) = crate::backups::take(target)? {
            eprintln!("backed up the old file as {}", b.display());
        }
    }
    // Files the game checks against the save, and what to put back if one can't be written.
    let companions = formats::companion_files(target, bytes);
    let before = if companions.is_empty() { None } else { Some(crate::backups::Kept::read(target).map_err(|e| format!("{}: {e}", target.display()))?) };
    // Written beside, flushed, read back and checked, then renamed over it (through a link).
    crate::backups::write_verified(target, bytes, true).map_err(|e| format!("{}: {e}", target.display()))?;
    if let Some(before) = before {
        crate::backups::write_companions(&before, companions, !no_backup).map_err(|e| {
            let stuck: Vec<String> = e.stuck.iter().map(|p| p.display().to_string()).collect();
            if stuck.is_empty() {
                format!("{}: {}; nothing was changed", e.file.display(), e.error)
            } else {
                format!("{}: {}; these files couldn't be put back as they were and may not match each other now: {}", e.file.display(), e.error, stuck.join(", "))
            }
        })?;
    }
    eprintln!("wrote {} ({} bytes)", target.display(), bytes.len());
    if let Some(m) = crate::playerprefs::after_save(target)? {
        eprintln!("{m}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get_decode_encode() {
        let dir = std::env::temp_dir().join(format!("use_cli_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let save = dir.join("save.json");
        std::fs::write(&save, br#"{"gold": 10, "name": "A"}"#).unwrap();
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let p = save.to_string_lossy().to_string();
        assert_eq!(run(&s(&["set", &p, "/gold=999", "/name=Bo", "--no-backup"])), Some(0));
        let v: Value = serde_json::from_slice(&std::fs::read(&save).unwrap()).unwrap();
        assert_eq!(v, json!({"gold": 999, "name": "Bo"}));
        assert_eq!(run(&s(&["set", &p, "/missing=1", "--no-backup"])), Some(1));
        // A gzip-packed save: decode, edit, encode back.
        use std::io::Write;
        let gz = dir.join("save.dat");
        let mut e = flate2::write::GzEncoder::new(vec![], flate2::Compression::default());
        e.write_all(br#"{"hp": 5}"#).unwrap();
        std::fs::write(&gz, e.finish().unwrap()).unwrap();
        let g = gz.to_string_lossy().to_string();
        let plain = dir.join("plain.json");
        assert_eq!(run(&s(&["decode", &g, "-o", &plain.to_string_lossy()])), Some(0));
        std::fs::write(&plain, br#"{"hp": 50}"#).unwrap();
        assert_eq!(run(&s(&["encode", &g, &plain.to_string_lossy(), "--no-backup"])), Some(0));
        let (_, _, d) = open(&g, &parse(&[]).unwrap()).unwrap();
        assert_eq!(root(&d.content).unwrap()["hp"], json!(50));
        assert!(matches!(d.layers.first(), Some(formats::Layer::Gzip)));
        assert_eq!(run(&s(&["photo.png"])), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A fresh folder for one test, removed when it ends.
    struct Scratch(PathBuf);
    impl Scratch {
        fn new(tag: &str) -> Self {
            let d = std::env::temp_dir().join(format!("use-cli-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            std::fs::create_dir_all(&d).unwrap();
            Scratch(d)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    fn backed_up(path: &Path) -> Vec<Vec<u8>> {
        crate::backups::list(path).iter().map(|v| std::fs::read(&v.path).unwrap()).collect()
    }

    #[test]
    fn set_backs_up_the_save_and_a_refused_edit_writes_nothing() {
        let _g = crate::backups::tests::LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::backups::set_root(None);
        let s = Scratch::new("backup");
        let save = s.0.join("save.json");
        let original = br#"{"gold": 10}"#;
        std::fs::write(&save, original).unwrap();
        let p = save.to_string_lossy().to_string();
        assert_eq!(run(&args(&["set", &p, "/gold=11"])), Some(0));
        assert_eq!(backed_up(&save), vec![original.to_vec()]);
        let now = std::fs::read(&save).unwrap();
        // A pointer that isn't there: nothing written, nothing backed up.
        assert_eq!(run(&args(&["set", &p, "/gold=12", "/nope=1"])), Some(1));
        assert_eq!(std::fs::read(&save).unwrap(), now);
        assert_eq!(backed_up(&save).len(), 1);
        assert!(!s.0.join("save.json.saving").exists() && !s.0.join("save.json.partial").exists());
    }

    /// A bare word is text only where the save holds text: a mistyped number is refused rather
    /// than written into the save as text.
    #[test]
    fn a_mistyped_number_is_refused_not_stored_as_text() {
        let s = Scratch::new("typo");
        let save = s.0.join("save.json");
        let original = br#"{"gold": 10, "name": "Ann", "hard": false}"#;
        std::fs::write(&save, original).unwrap();
        let p = save.to_string_lossy().to_string();
        for typed in ["/gold=1,000", "/gold=99O", "/gold=1.000,5", "/hard=yes", "/gold=", "/name=Bo /gold=x"] {
            let edits: Vec<&str> = typed.split(' ').collect();
            let mut a = vec!["set", p.as_str()];
            a.extend(edits);
            a.push("--no-backup");
            assert_eq!(run(&args(&a)), Some(1), "{typed}");
            assert_eq!(std::fs::read(&save).unwrap(), original, "{typed}: nothing written");
        }
        // Text without quotes where the save holds text, and JSON anywhere, still work.
        assert_eq!(run(&args(&["set", &p, "/name=Bo Bo", "/gold=1000", "/hard=true", "--no-backup"])), Some(0));
        let v: Value = serde_json::from_slice(&std::fs::read(&save).unwrap()).unwrap();
        assert_eq!(v, json!({"gold": 1000, "name": "Bo Bo", "hard": true}));
        assert_eq!(run(&args(&["set", &p, "/gold=\"lots\"", "--no-backup"])), Some(0), "quoted text is taken as asked");
    }

    #[test]
    fn decoding_over_an_existing_file_backs_it_up_first() {
        let _g = crate::backups::tests::LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::backups::set_root(None);
        let s = Scratch::new("decode-over");
        use std::io::Write;
        let mut e = flate2::write::GzEncoder::new(vec![], flate2::Compression::default());
        e.write_all(br#"{"hp": 5}"#).unwrap();
        let packed = e.finish().unwrap();
        let save = s.0.join("save.dat");
        std::fs::write(&save, &packed).unwrap();
        let p = save.to_string_lossy().to_string();
        // `-o` names the save itself: the packed save must not be lost.
        assert_eq!(run(&args(&["decode", &p, "-o", &p])), Some(0));
        let v: Value = serde_json::from_slice(&std::fs::read(&save).unwrap()).unwrap();
        assert_eq!(v, json!({"hp": 5}));
        assert_eq!(backed_up(&save), vec![packed]);
    }

    #[test]
    fn a_companion_that_cant_be_written_leaves_the_save_as_it_was() {
        let _g = crate::backups::tests::LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::backups::set_root(None);
        let s = Scratch::new("companion");
        let save = s.0.join("data0001.bin");
        let original = br#"{"gold": 1}"#;
        std::fs::write(&save, original).unwrap();
        let mut slot = b"SAVE0001".to_vec();
        slot.resize(0x40, 0);
        std::fs::write(s.0.join("data0001.binslot"), &slot).unwrap();
        std::fs::create_dir_all(s.0.join("data0001.binslot.partial").join("x")).unwrap();
        let p = save.to_string_lossy().to_string();
        assert_eq!(run(&args(&["set", &p, "/gold=2"])), Some(1));
        assert_eq!(std::fs::read(&save).unwrap(), original);
        assert_eq!(std::fs::read(s.0.join("data0001.binslot")).unwrap(), slot);
        std::fs::remove_dir_all(s.0.join("data0001.binslot.partial")).unwrap();
        assert_eq!(run(&args(&["set", &p, "/gold=2"])), Some(0));
        use md5::Digest;
        assert_eq!(std::fs::read(s.0.join("data0001.binslot")).unwrap()[0x18..0x28], md5::Md5::digest(std::fs::read(&save).unwrap())[..]);
    }

    #[test]
    fn decode_with_no_backup_makes_none() {
        let _g = crate::backups::tests::LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::backups::set_root(None);
        let s = Scratch::new("decode-no-backup");
        let save = s.0.join("save.json");
        std::fs::write(&save, br#"{"hp": 5}"#).unwrap();
        let out = s.0.join("out.json");
        std::fs::write(&out, b"old export").unwrap();
        let (p, o) = (save.to_string_lossy().to_string(), out.to_string_lossy().to_string());
        assert_eq!(run(&args(&["decode", &p, "-o", &o, "--no-backup"])), Some(0));
        assert!(crate::backups::list(&out).is_empty());
        assert!(!s.0.join(crate::backups::BESIDE).exists());
        let v: Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
        assert_eq!(v, json!({"hp": 5}));
    }

    #[cfg(unix)]
    #[test]
    fn writing_through_a_linked_save_keeps_the_link() {
        let _g = crate::backups::tests::LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::backups::set_root(None);
        let s = Scratch::new("link");
        std::fs::create_dir_all(s.0.join("synced")).unwrap();
        let real = s.0.join("synced").join("save.json");
        std::fs::write(&real, br#"{"gold": 1}"#).unwrap();
        let save = s.0.join("save.json");
        std::os::unix::fs::symlink(&real, &save).unwrap();
        assert_eq!(run(&args(&["set", &save.to_string_lossy(), "/gold=2"])), Some(0));
        assert!(std::fs::symlink_metadata(&save).unwrap().file_type().is_symlink());
        let v: Value = serde_json::from_slice(&std::fs::read(&real).unwrap()).unwrap();
        assert_eq!(v["gold"], json!(2));
    }

    fn gold(save: &Path) -> i64 {
        serde_json::from_slice::<Value>(&std::fs::read(save).unwrap()).unwrap()["gold"].as_i64().unwrap()
    }

    #[test]
    fn backup_backups_and_restore_round_trip() {
        let _g = crate::backups::tests::LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::backups::set_root(None);
        let s = Scratch::new("brr");
        let save = s.0.join("save.json");
        std::fs::write(&save, br#"{"gold": 10}"#).unwrap();
        let p = save.to_string_lossy().to_string();
        assert_eq!(run(&args(&["backup", &p, "--name", "before the boss"])), Some(0));
        assert_eq!(run(&args(&["backups", &p])), Some(0));
        assert_eq!(run(&args(&["backups", &p, "--json"])), Some(0));
        assert!(crate::backups::list(&save).iter().any(|v| v.name.as_deref() == Some("before the boss")));
        assert_eq!(run(&args(&["set", &p, "/gold=99"])), Some(0));
        assert_eq!(gold(&save), 99);
        let slot = crate::backups::list(&save).iter().position(|v| v.name.is_some()).unwrap() + 1;
        assert_eq!(run(&args(&["restore", &p, &slot.to_string()])), Some(0));
        assert_eq!(gold(&save), 10);
        // The save that was replaced is kept too, so the restore can be undone.
        assert!(crate::backups::list(&save).iter().any(|v| std::fs::read(&v.path).map(|b| b == br#"{"gold": 99}"#.to_vec() || String::from_utf8_lossy(&b).contains("99")).unwrap_or(false)));
        // A number that isn't there writes nothing.
        assert_eq!(run(&args(&["restore", &p, "99"])), Some(2));
        assert_eq!(run(&args(&["backup", &s.0.join("none.json").to_string_lossy()])), Some(1));
        assert_eq!(gold(&save), 10);
    }

    #[test]
    fn apply_a_preset_file_and_a_ready_made_preset() {
        let _g = crate::backups::tests::LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::backups::set_root(None);
        let s = Scratch::new("apply");
        let save = s.0.join("file1.json");
        std::fs::write(&save, br#"{"party":{"_gold":10,"_items":{"1":3,"2":120}},"gold":5}"#).unwrap();
        let p = save.to_string_lossy().to_string();
        let preset = s.0.join("rich.json");
        std::fs::write(&preset, r#"{"preset":"Rich","values":[{"path":"/gold","value":500},{"path":"/nope","value":1}]}"#).unwrap();
        // A dry run changes nothing.
        assert_eq!(run(&args(&["apply", &p, &preset.to_string_lossy(), "--dry-run"])), Some(0));
        assert_eq!(gold(&save), 5);
        assert_eq!(run(&args(&["apply", &p, &preset.to_string_lossy()])), Some(0));
        assert_eq!(gold(&save), 500);
        assert_eq!(backed_up(&save).len(), 1, "the old file was backed up first");
        // Ready-made, by name: items below 99 go up, others and the gold already above stay.
        assert_eq!(run(&args(&["apply", &p, "carried items to 99 (rpg maker mv / mz)"])), Some(0));
        let v: Value = serde_json::from_slice(&std::fs::read(&save).unwrap()).unwrap();
        assert_eq!(v["party"]["_items"]["1"], 99);
        assert_eq!(v["party"]["_items"]["2"], 120);
        // Nothing to change: the file isn't rewritten.
        let before = std::fs::read(&save).unwrap();
        assert_eq!(run(&args(&["apply", &p, "carried items to 99 (rpg maker mv / mz)"])), Some(0));
        assert_eq!(std::fs::read(&save).unwrap(), before);
        // Unknown preset: error, nothing written.
        assert_eq!(run(&args(&["apply", &p, "no such preset"])), Some(1));
        assert_eq!(std::fs::read(&save).unwrap(), before);
        assert_eq!(run(&args(&["presets", "--json"])), Some(0));
    }

}
