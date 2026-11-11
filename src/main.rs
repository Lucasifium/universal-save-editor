#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(non_snake_case)] // the executable is named UniversalSaveEditor, not snake_case

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        // A command (`info`, `set`, `decode`…) runs without opening the window.
        attach_console();
        if let Some(code) = universal_save_editor::run_cli(&args) {
            std::process::exit(code);
        }
    }
    // Otherwise the first argument is a file to open (or one dropped onto the program).
    universal_save_editor::run_desktop(std::env::args_os().nth(1).map(std::path::PathBuf::from))
}

/// The release build is a window program on Windows, so it has no console of its own: borrow
/// the one it was started from, so command output shows up there.
#[cfg(windows)]
fn attach_console() {
    extern "system" {
        fn AttachConsole(process: u32) -> i32;
    }
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(not(windows))]
fn attach_console() {}
