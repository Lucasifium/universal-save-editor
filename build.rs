// Build script: draws the app icon (src/icon.rs) and embeds it, with the program's name, into the
// Windows executable. For Android and macOS builds it also writes the icon as PNGs into
// OUT_DIR/icons, which the packaging scripts pick up.

#[path = "src/icon.rs"]
mod icon;

fn main() {
    println!("cargo:rerun-if-changed=src/icon.rs");
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
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
        res.set("FileDescription", "UniversalSaveEditor");
        res.set("ProductName", "UniversalSaveEditor");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed icon: {e}");
        }
    }
}
