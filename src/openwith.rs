//! "Open with" on Windows: lets Explorer list the editor for a few save file types that
//! are distinctive to one game (never the very common ones like `.sav` or `.dat`). It only
//! adds the editor to each type's "Open with" list for the current user; it never becomes
//! the default program and never touches the machine-wide settings. Off until the player
//! turns it on in Preferences, and turning it off removes every entry again.

#![cfg_attr(not(windows), allow(dead_code))]

/// File types that mean one family of games, so listing the editor for them is not noise.
pub const EXTENSIONS: [&str; 10] = ["sl2", "rpgsave", "rmmzsave", "rxdata", "lsv", "ess", "fos", "d2s", "es3", "sii"];

/// Name of the program entry the file types point to.
pub const PROG_ID: &str = "UniversalSaveEditor.Save";

const CLASSES: &str = r"HKCU\Software\Classes";

/// The `reg.exe` argument lists that add the entries for the program at `exe`.
pub fn register_commands(exe: &str) -> Vec<Vec<String>> {
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    let mut out = vec![
        s(&["add", &format!(r"{CLASSES}\{PROG_ID}"), "/ve", "/d", "Game save (Universal Save Editor)", "/f"]),
        s(&["add", &format!(r"{CLASSES}\{PROG_ID}\DefaultIcon"), "/ve", "/d", &format!("\"{exe}\",0"), "/f"]),
        s(&["add", &format!(r"{CLASSES}\{PROG_ID}\shell\open\command"), "/ve", "/d", &format!("\"{exe}\" \"%1\""), "/f"]),
    ];
    for e in EXTENSIONS {
        out.push(s(&["add", &format!(r"{CLASSES}\.{e}\OpenWithProgids"), "/v", PROG_ID, "/t", "REG_NONE", "/f"]));
    }
    out
}

/// The `reg.exe` argument lists that remove everything `register_commands` added.
pub fn unregister_commands() -> Vec<Vec<String>> {
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    let mut out: Vec<Vec<String>> = EXTENSIONS.iter().map(|e| s(&["delete", &format!(r"{CLASSES}\.{e}\OpenWithProgids"), "/v", PROG_ID, "/f"])).collect();
    out.push(s(&["delete", &format!(r"{CLASSES}\{PROG_ID}"), "/f"]));
    out
}

#[cfg(windows)]
fn run(cmds: Vec<Vec<String>>, ignore_missing: bool) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    for c in cmds {
        let out = std::process::Command::new("reg").args(&c).creation_flags(0x0800_0000).output().map_err(|e| e.to_string())?;
        if !out.status.success() && !ignore_missing {
            return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
        }
    }
    Ok(())
}

/// Adds the editor to the "Open with" list of the file types above, for this user.
#[cfg(windows)]
pub fn register() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    run(register_commands(&exe.display().to_string()), false)
}

/// Removes the entries again (entries that are already gone are not an error).
#[cfg(windows)]
pub fn unregister() -> Result<(), String> {
    run(unregister_commands(), true)
}

#[cfg(not(windows))]
pub fn register() -> Result<(), String> {
    Err("only on Windows".into())
}

#[cfg(not(windows))]
pub fn unregister() -> Result<(), String> {
    Err("only on Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_distinctive_types_and_only_the_current_user() {
        for common in ["sav", "dat", "bin", "json", "save", "txt", "xml"] {
            assert!(!EXTENSIONS.contains(&common), "{common} is too common to claim");
        }
        let all: Vec<_> = register_commands(r"C:\Tools\UniversalSaveEditor.exe").into_iter().chain(unregister_commands()).collect();
        assert!(all.iter().all(|c| c.iter().any(|a| a.starts_with(r"HKCU\Software\Classes"))));
        assert!(all.iter().all(|c| !c.iter().any(|a| a.starts_with("HKLM"))));
    }

    #[test]
    fn open_command_quotes_the_program_and_the_file() {
        let cmds = register_commands(r"C:\My Games\Save Editor.exe");
        let open = cmds.iter().find(|c| c[1].ends_with(r"shell\open\command")).unwrap();
        assert_eq!(open[4], r#""C:\My Games\Save Editor.exe" "%1""#);
        // Adds only to "Open with"; never writes a type's default program.
        assert!(cmds.iter().filter(|c| c[1].contains(r"\.")).all(|c| c[1].ends_with("OpenWithProgids")));
        assert_eq!(register_commands("x").len(), 3 + EXTENSIONS.len());
        assert_eq!(unregister_commands().len(), EXTENSIONS.len() + 1);
    }
}
