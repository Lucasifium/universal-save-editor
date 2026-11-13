//! Universal Save Editor: the app, shared by the desktop executable, the Android library and the
//! browser version (`web.rs`). `run_desktop` starts the window; the readers in
//! `fmt/` decide what each save is.

#[cfg(test)]
mod bughunt;
#[cfg(test)]
mod site;
mod app;
mod apollo;
mod backups;
#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
mod cli;
mod file_browser;
mod fmt;
mod formats;
mod games;
mod game_ui;
mod tree;
mod hex_view;
mod icon;
mod json_view;
mod names;
mod oodle;
mod pickle;
mod par;
mod platform;
mod presets;
mod preview;
#[cfg(not(target_arch = "wasm32"))]
mod playerprefs;
mod renpy_sign;
mod risks;
mod running;
mod settings;
mod steamid;
mod theme;
mod values_view;
mod vfs;
mod xgp;
#[cfg(target_arch = "wasm32")]
mod web;
mod widgets;

pub use platform::APP_NAME;

/// Command-line use (`UniversalSaveEditor info <save>`, `set`, `decode`, …; see `cli.rs`).
/// Returns the exit code, or `None` when the arguments mean "open the window".
#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
pub fn run_cli(args: &[String]) -> Option<i32> {
    cli::run(args)
}

#[cfg(not(target_arch = "wasm32"))]
/// Desktop entry point. `initial`: a file passed on the command line (or dropped onto the program).
pub fn run_desktop(initial: Option<std::path::PathBuf>) -> eframe::Result {
    let icon = eframe::egui::IconData { rgba: icon::rgba(64), width: 64, height: 64 };
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(platform::DISPLAY_NAME)
            .with_app_id(APP_NAME)
            .with_inner_size([1150.0, 760.0])
            .with_min_inner_size([700.0, 400.0])
            .with_drag_and_drop(true)
            .with_icon(icon),
        ..Default::default()
    };
    eframe::run_native(APP_NAME, options, Box::new(|cc| Ok(Box::new(app::App::new(cc, initial)))))
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(android: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;
    if let Some(dir) = android.internal_data_path() {
        // A crash on a phone is otherwise invisible; keep the last one next to the settings.
        let log = dir.join("last_crash.txt");
        std::panic::set_hook(Box::new(move |info| {
            let _ = std::fs::write(&log, format!("{info}\n{}", std::backtrace::Backtrace::force_capture()));
        }));
        platform::set_android_data_dir(dir);
    }
    let _ = platform::android::APP.set(android.clone());
    let options = eframe::NativeOptions {
        event_loop_builder: Some(Box::new(move |b| {
            b.with_android_app(android);
        })),
        ..Default::default()
    };
    let initial = platform::android::intent_path();
    let _ = eframe::run_native(APP_NAME, options, Box::new(|cc| Ok(Box::new(app::App::new(cc, initial)))));
}
