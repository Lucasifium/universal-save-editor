#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(non_snake_case)] // the executable is named SaveEditorOffline

fn main() -> eframe::Result {
    // Optional: pass a file path on the command line (or drop a file onto the program).
    save_editor_offline::run_desktop(std::env::args_os().nth(1).map(std::path::PathBuf::from))
}
