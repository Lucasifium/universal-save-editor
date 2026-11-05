//! SaveEditorOffline: the app, shared by the desktop executable and the Android library.

#[cfg(test)]
mod bughunt;
mod app;
mod backups;
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
mod renpy_sign;
mod running;
mod settings;
mod theme;
mod values_view;
mod widgets;

pub use platform::APP_NAME;

/// Desktop entry point. `initial`: a file passed on the command line (or dropped onto the program).
pub fn run_desktop(initial: Option<std::path::PathBuf>) -> eframe::Result {
    let icon = eframe::egui::IconData { rgba: icon::rgba(64), width: 64, height: 64 };
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(APP_NAME)
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
