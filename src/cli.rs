//! The command line: the same readers and writers as the window, for scripts and batch edits.
//!
//! ```text
//! UniversalSaveEditor info   <save> [--json]
//! UniversalSaveEditor list   <save> [--json]                  every value, by pointer
//! UniversalSaveEditor get    <save> <pointer>...
//! UniversalSaveEditor set    <save> <pointer>=<json>... [-o <out>]
//! UniversalSaveEditor decode <save> [-o <out>]                the unpacked data
//! UniversalSaveEditor encode <save> <edited data> [-o <out>]  pack edited data like <save>
//! UniversalSaveEditor games  [--json]                         the games the editor knows
//! ```
//!
//! Writing is as careful as the window: the new file must decode to exactly what was asked
//! for before anything is written, the old file is backed up first (File › Restore previous
//! version lists the copies), and the write is atomic. `--password <p>` decrypts like
//! Tools › Decrypt with password.

use crate::formats::{self, Content, Options};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const COMMANDS: [&str; 8] = ["info", "list", "get", "set", "decode", "encode", "games", "help"];

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
    no_backup: bool,
}

fn parse(args: &[String]) -> Result<Args, String> {
    let mut a = Args { pos: vec![], json: false, out: None, password: None, no_backup: false };
    let mut it = args.iter();
    while let Some(x) = it.next() {
        match x.as_str() {
            "--json" => a.json = true,
            "--no-backup" => a.no_backup = true,
            "-o" | "--out" => a.out = Some(PathBuf::from(it.next().ok_or("usage: -o needs a file")?)),
            "--password" => a.password = Some(it.next().ok_or("usage: --password needs a value")?.clone()),
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
         UniversalSaveEditor games  [--json]                the games the editor knows\n\n\
         Options: --password <p> (encrypted saves), --json (machine-readable output).\n\
         Pointers are JSON pointers as shown by `list` (for example /money or /party/0/hp).\n\
         Values are JSON: 999, 2.5, true, \"text\".\n\
         Writing checks the new file reads back exactly, backs up the old one, and replaces it atomically.",
        crate::platform::DISPLAY_NAME,
        env!("CARGO_PKG_VERSION")
    );
}

fn open(path: &str, password: &Option<String>) -> Result<(PathBuf, Options, formats::Decoded), String> {
    let p = PathBuf::from(path);
    let raw = std::fs::read(&p).map_err(|e| format!("{path}: {e}"))?;
    let mut opts = Options::for_path(&p);
    opts.password = password.clone();
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
            let (_, _, d) = open(f, &a.password)?;
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
            }
        }
        "list" => {
            let f = a.pos.first().ok_or("usage: list <save>")?;
            let (_, _, d) = open(f, &a.password)?;
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
            let (_, _, d) = open(f, &a.password)?;
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
            let (path, opts, mut d) = open(f, &a.password)?;
            for e in edits {
                let (ptr, text) = e.split_once('=').ok_or_else(|| format!("usage: {e}: write pointer=value"))?;
                // Bare words are taken as text, so `/name=Ann` works without quotes.
                let v: Value = serde_json::from_str(text).unwrap_or_else(|_| Value::String(text.to_string()));
                let old = root(&d.content).and_then(|r| r.pointer(ptr)).cloned().ok_or_else(|| format!("{ptr}: not in this save"))?;
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
            let bytes = formats::encode(&d.layers, &d.content)?;
            let check = formats::decode_with(&bytes, &opts);
            if !formats::same_layers(&check.layers, &d.layers) || !formats::verified(&check.content, &d.content) {
                return Err("the new file didn't read back identically; nothing was written".into());
            }
            write(&path, a.out.as_deref(), &bytes, a.no_backup)?;
        }
        "decode" => {
            let f = a.pos.first().ok_or("usage: decode <save> [-o <out>]")?;
            let (_, _, d) = open(f, &a.password)?;
            let bytes = d.content.to_bytes()?;
            match &a.out {
                Some(o) => std::fs::write(o, &bytes).map_err(|e| format!("{}: {e}", o.display()))?,
                None => {
                    use std::io::Write;
                    std::io::stdout().write_all(&bytes).map_err(|e| e.to_string())?;
                }
            }
        }
        "encode" => {
            let [f, data] = &a.pos[..] else { return Err("usage: encode <save> <edited data> [-o <out>]".into()) };
            let (path, opts, d) = open(f, &a.password)?;
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

/// Writes `bytes` to `out` (or over `path`), backing up whatever is replaced, atomically.
fn write(path: &Path, out: Option<&Path>, bytes: &[u8], no_backup: bool) -> Result<(), String> {
    let target = out.unwrap_or(path);
    if target.exists() && !no_backup {
        if let Some(b) = crate::backups::take(target)? {
            eprintln!("backed up the old file as {}", b.display());
        }
    }
    let mut tmp = target.as_os_str().to_owned();
    tmp.push(".saving");
    let tmp = PathBuf::from(tmp);
    if let Err(e) = std::fs::write(&tmp, bytes).and_then(|_| std::fs::rename(&tmp, target)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("{}: {e}", target.display()));
    }
    for (other, data) in formats::companion_files(target, bytes) {
        if !no_backup {
            let _ = crate::backups::take(&other);
        }
        std::fs::write(&other, data).map_err(|e| format!("{}: {e}", other.display()))?;
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
        let (_, _, d) = open(&g, &None).unwrap();
        assert_eq!(root(&d.content).unwrap()["hp"], json!(50));
        assert!(matches!(d.layers.first(), Some(formats::Layer::Gzip)));
        assert_eq!(run(&s(&["photo.png"])), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
