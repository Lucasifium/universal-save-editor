// Build script: draws the app icon (src/icon.rs) and embeds it, with the program's name, into the
// Windows executable. For Android and macOS builds it also writes the icon as PNGs into
// OUT_DIR/icons, which the packaging scripts pick up.

#[path = "src/icon.rs"]
mod icon;

/// Lists the game definitions in data/games/*.toml (sorted by file name) so the program embeds
/// them: adding a game is adding a file there, no Rust change.
fn embed_game_definitions(out_dir: &std::path::Path) {
    let dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("data").join("games");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
        .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "toml")).collect())
        .unwrap_or_default();
    files.sort();
    let mut src = String::from("pub static GAME_DEFINITIONS: &[(&str, &str)] = &[\n");
    for f in &files {
        println!("cargo:rerun-if-changed={}", f.display());
        let name = f.file_name().unwrap().to_string_lossy();
        src.push_str(&format!("    ({name:?}, include_str!({:?})),\n", f.to_string_lossy()));
    }
    src.push_str("];\n");
    std::fs::write(out_dir.join("game_definitions.rs"), src).expect("write game definitions list");
}

fn main() {
    println!("cargo:rerun-if-changed=src/icon.rs");
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    embed_game_definitions(&out_dir);
    let ico_path = out_dir.join("app.ico");
    std::fs::write(&ico_path, icon::ico(&[16, 24, 32, 48, 64, 128, 256])).expect("write icon");

    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let png_sizes: &[u32] = match os.as_str() {
        "android" => &[48, 72, 96, 144, 192],
        "macos" => &[16, 32, 64, 128, 256, 512, 1024],
        _ => &[],
    };
    if !png_sizes.is_empty() {
        let dir = out_dir.join("icons");
        std::fs::create_dir_all(&dir).expect("icons dir");
        for &s in png_sizes {
            std::fs::write(dir.join(format!("{s}.png")), icon::png(s)).expect("write png");
        }
    }

    if os == "windows" {
        let mut res = winresource::WindowsResource::new();
        res.set_icon(ico_path.to_str().unwrap());
        res.set("FileDescription", "Universal Save Editor");
        res.set("ProductName", "Universal Save Editor");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed icon: {e}");
        }
    }
}
