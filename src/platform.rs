//! The few things that differ between Windows, macOS, Linux and Android.

use std::path::{Path, PathBuf};

pub const APP_NAME: &str = "UniversalSaveEditor";
/// The name people see (window title, start screen); `APP_NAME` names folders and settings.
pub const DISPLAY_NAME: &str = "Universal Save Editor";

#[cfg(target_os = "android")]
static ANDROID_DATA: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

#[cfg(target_os = "android")]
pub fn set_android_data_dir(p: PathBuf) {
    let _ = ANDROID_DATA.set(p);
}

/// Where the app keeps its own settings.
#[cfg(target_os = "android")]
pub fn config_dir() -> Option<PathBuf> {
    ANDROID_DATA.get().cloned()
}

/// Where the app keeps its own settings.
#[cfg(not(target_os = "android"))]
pub fn config_dir() -> Option<PathBuf> {
    let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    if cfg!(windows) {
        env("APPDATA").map(|d| windows_dir(&d))
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|h| h.join("Library").join("Application Support").join(APP_DIR))
    } else {
        env("XDG_CONFIG_HOME").or_else(|| env("HOME").map(|h| h.join(".config"))).map(|d| d.join(APP_DIR))
    }
}

/// %APPDATA%\UniversalSaveEditor, bringing over the settings from an older folder name.
#[cfg(not(target_os = "android"))]
fn windows_dir(appdata: &Path) -> PathBuf {
    let dir = appdata.join(APP_DIR);
    let old = appdata.join("SaveEditor").join("settings.json");
    if !dir.exists() && old.is_file() {
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::copy(old, dir.join("settings.json"));
    }
    dir
}

/// Label for the "show this file in the file manager" command, if there is one.
pub fn reveal_label() -> Option<&'static str> {
    if cfg!(windows) {
        Some("Show file in Explorer")
    } else if cfg!(target_os = "macos") {
        Some("Show file in Finder")
    } else if cfg!(target_os = "android") {
        None
    } else {
        Some("Open containing folder")
    }
}

pub fn reveal(path: &Path) {
    use std::process::Command;
    let _ = if cfg!(windows) {
        Command::new("explorer").arg("/select,").arg(path).spawn()
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg("-R").arg(path).spawn()
    } else {
        Command::new("xdg-open").arg(path.parent().unwrap_or(Path::new("/"))).spawn()
    };
}

/// Android: the app can only read and write saves anywhere on the device once the user
/// grants "All files access".
#[cfg(target_os = "android")]
pub mod android {
    use jni::objects::{JObject, JValue};
    use jni::JavaVM;
    use std::sync::OnceLock;

    pub static APP: OnceLock<winit::platform::android::activity::AndroidApp> = OnceLock::new();

    fn with_env<T>(f: impl FnOnce(&mut jni::JNIEnv, &JObject) -> jni::errors::Result<T>) -> Option<T> {
        let app = APP.get()?;
        let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as *mut _) }.ok()?;
        let mut env = vm.attach_current_thread().ok()?;
        let activity = unsafe { JObject::from_raw(app.activity_as_ptr() as jni::sys::jobject) };
        let r = f(&mut env, &activity);
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
        }
        r.ok()
    }

    fn sdk() -> i32 {
        with_env(|env, _| env.get_static_field("android/os/Build$VERSION", "SDK_INT", "I")?.i()).unwrap_or(0)
    }

    pub fn has_file_access() -> bool {
        if sdk() < 30 {
            // Older Android: the storage permission granted at install / in Settings.
            return with_env(|env, act| {
                let p = env.new_string("android.permission.WRITE_EXTERNAL_STORAGE")?;
                Ok(env.call_method(act, "checkSelfPermission", "(Ljava/lang/String;)I", &[JValue::Object(&p)])?.i()? == 0)
            })
            .unwrap_or(false);
        }
        with_env(|env, _| env.call_static_method("android/os/Environment", "isExternalStorageManager", "()Z", &[])?.z()).unwrap_or(false)
    }

    /// Opens the system screen where the user allows file access.
    pub fn request_file_access() {
        let modern = sdk() >= 30;
        with_env(|env, act| {
            if !modern {
                let cls = env.find_class("java/lang/String")?;
                let p = env.new_string("android.permission.WRITE_EXTERNAL_STORAGE")?;
                let arr = env.new_object_array(1, cls, &p)?;
                env.call_method(act, "requestPermissions", "([Ljava/lang/String;I)V", &[JValue::Object(&arr), JValue::Int(1)])?;
                return Ok(());
            }
            let pkg = env.call_method(act, "getPackageName", "()Ljava/lang/String;", &[])?.l()?;
            let prefix = env.new_string("package:")?;
            let uri_s = env.call_method(&prefix, "concat", "(Ljava/lang/String;)Ljava/lang/String;", &[JValue::Object(&pkg)])?.l()?;
            let uri = env.call_static_method("android/net/Uri", "parse", "(Ljava/lang/String;)Landroid/net/Uri;", &[JValue::Object(&uri_s)])?.l()?;
            let action = env.new_string("android.settings.MANAGE_APP_ALL_FILES_ACCESS_PERMISSION")?;
            let intent = env.new_object("android/content/Intent", "(Ljava/lang/String;Landroid/net/Uri;)V", &[JValue::Object(&action), JValue::Object(&uri)])?;
            env.call_method(act, "startActivity", "(Landroid/content/Intent;)V", &[JValue::Object(&intent)])?;
            Ok(())
        });
    }

    /// The file the app was started with ("Open with", or `am start -d file:///…`), if it's
    /// a plain file path.
    pub fn intent_path() -> Option<std::path::PathBuf> {
        with_env(|env, act| {
            let intent = env.call_method(act, "getIntent", "()Landroid/content/Intent;", &[])?.l()?;
            let uri = env.call_method(&intent, "getData", "()Landroid/net/Uri;", &[])?.l()?;
            if uri.is_null() {
                return Ok(None);
            }
            let scheme = env.call_method(&uri, "getScheme", "()Ljava/lang/String;", &[])?.l()?;
            let path = env.call_method(&uri, "getPath", "()Ljava/lang/String;", &[])?.l()?;
            if scheme.is_null() || path.is_null() {
                return Ok(None);
            }
            let scheme: String = env.get_string(&scheme.into())?.into();
            let path: String = env.get_string(&path.into())?.into();
            Ok((scheme == "file").then(|| path.into()))
        })
        .flatten()
    }

    /// Height in pixels of what covers the bottom of the window (a tablet's taskbar, the
    /// navigation buttons): the window minus the content area Android reports.
    pub fn bottom_inset_px() -> f32 {
        let Some(app) = APP.get() else { return 0.0 };
        let Some(win) = app.native_window() else { return 0.0 };
        let (r, h) = (app.content_rect(), win.height());
        if r.bottom > 0 && r.bottom < h {
            (h - r.bottom) as f32
        } else {
            0.0
        }
    }

    /// Shows or hides the on-screen keyboard (MainActivity.java's text-input view).
    pub fn set_keyboard(show: bool) {
        with_env(|env, act| env.call_method(act, "setKeyboard", "(Z)V", &[JValue::Bool(show.into())]).map(|_| ()));
    }

    /// What the on-screen keyboard typed, waiting for the next frame.
    pub enum Typed {
        Text(String),
        Key(eframe::egui::Key),
    }

    static TYPED: std::sync::Mutex<Vec<Typed>> = std::sync::Mutex::new(Vec::new());
    pub static CTX: OnceLock<eframe::egui::Context> = OnceLock::new();

    fn push(t: Typed) {
        if let Ok(mut q) = TYPED.lock() {
            q.push(t);
        }
        if let Some(c) = CTX.get() {
            c.request_repaint();
        }
    }

    /// Adds the keyboard's input to this frame's events.
    pub fn take_typed(raw: &mut eframe::egui::RawInput) {
        use eframe::egui::Event;
        let Ok(mut q) = TYPED.lock() else { return };
        for t in q.drain(..) {
            match t {
                Typed::Text(s) => raw.events.push(Event::Text(s)),
                Typed::Key(key) => {
                    for pressed in [true, false] {
                        raw.events.push(Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: Default::default() });
                    }
                }
            }
        }
    }

    #[no_mangle]
    extern "system" fn Java_com_universalsaveeditor_app_MainActivity_nativeText(mut env: jni::JNIEnv, _: jni::objects::JClass, text: jni::objects::JString) {
        if let Ok(s) = env.get_string(&text) {
            push(Typed::Text(s.into()));
        }
    }

    #[no_mangle]
    extern "system" fn Java_com_universalsaveeditor_app_MainActivity_nativeKey(_: jni::JNIEnv, _: jni::objects::JClass, code: jni::sys::jint) {
        use eframe::egui::Key;
        // android.view.KeyEvent key codes.
        let key = match code {
            67 => Key::Backspace,
            112 => Key::Delete,
            66 | 160 => Key::Enter,
            21 => Key::ArrowLeft,
            22 => Key::ArrowRight,
            19 => Key::ArrowUp,
            20 => Key::ArrowDown,
            61 => Key::Tab,
            _ => return,
        };
        push(Typed::Key(key));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn old_settings_are_copied_once() {
        let base = std::env::temp_dir().join(format!("seo_settings_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("SaveEditor")).unwrap();
        std::fs::write(base.join("SaveEditor").join("settings.json"), "{\"dark\":false}").unwrap();
        let dir = super::windows_dir(&base);
        assert_eq!(dir, base.join("UniversalSaveEditor"));
        assert_eq!(std::fs::read_to_string(dir.join("settings.json")).unwrap(), "{\"dark\":false}");
        // Later changes in the new folder are kept, not overwritten by the old file.
        std::fs::write(dir.join("settings.json"), "{}").unwrap();
        super::windows_dir(&base);
        assert_eq!(std::fs::read_to_string(dir.join("settings.json")).unwrap(), "{}");
        let _ = std::fs::remove_dir_all(&base);
    }
}

/// Steam's `userdata` folders on this machine (one subfolder per account that has played).
pub fn steam_userdata_dirs() -> Vec<PathBuf> {
    let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    let mut roots: Vec<PathBuf> = vec![];
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        if let Ok(out) = std::process::Command::new("reg")
            .args(["query", r"HKCU\Software\Valve\Steam", "/v", "SteamPath"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            if let Some(p) = text.lines().find_map(|l| l.split("REG_SZ").nth(1)) {
                // Steam writes it with forward slashes ("f:/steam").
                roots.push(PathBuf::from(p.trim().replace('/', "\\")));
            }
        }
        if let Some(pf) = env("ProgramFiles(x86)") {
            roots.push(pf.join("Steam"));
        }
    }
    if let Some(home) = env("HOME") {
        roots.push(home.join(".local/share/Steam"));
        roots.push(home.join(".steam/steam"));
        roots.push(home.join("Library/Application Support/Steam"));
    }
    let mut out: Vec<PathBuf> = vec![];
    for r in roots {
        let u = r.join("userdata");
        if u.is_dir() && !out.contains(&u) {
            out.push(u);
        }
    }
    out
}
