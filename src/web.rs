//! The browser version: started by `web/index.html`, it runs entirely on the visitor's
//! computer. Files come in through the browser's file picker or by dropping them on the page,
//! and go back out as downloads; nothing is uploaded anywhere. Where the browser allows it
//! (Chrome and Edge), a file opened with "Open to save in place" is written back to the same
//! file instead, once the browser has asked the player; `web/fsa.js` does that side.

use eframe::egui;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

thread_local! {
    /// Files the picker has finished reading, waiting for the app to take them.
    static PICKED: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
    /// Files picked with "Open to save in place": the browser's handle to each, until the app
    /// says whether the file that opens may be written back through it (`settle_handle`).
    static PENDING: RefCell<HashMap<PathBuf, JsValue>> = RefCell::new(HashMap::new());
    /// The handles documents save back through, by the id the document holds. Only the
    /// document that opened a file has its id; nothing looks a handle up by file name.
    static HANDLES: RefCell<HashMap<u64, JsValue>> = RefCell::new(HashMap::new());
    static NEXT_ID: Cell<u64> = const { Cell::new(1) };
    /// What reading a file again through its handle came to, for the app to take.
    static REREAD: RefCell<Option<(u64, Result<Vec<u8>, String>)>> = const { RefCell::new(None) };
    /// The next save is a download even if the file has a handle ("Save a copy as").
    static DOWNLOAD_NEXT: Cell<bool> = const { Cell::new(false) };
    /// The kept originals, once the browser has listed them.
    static ORIGINALS: RefCell<Option<Result<Vec<Original>, String>>> = const { RefCell::new(None) };
    /// What went wrong picking a file in place, for the app to show.
    static PICK_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
}

// web/fsa.js, loaded by the page before the app starts. Each call is `catch`, so a page
// without it only loses saving in place.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = saveEditorFiles, js_name = supported)]
    fn js_supported() -> Result<bool, JsValue>;
    #[wasm_bindgen(catch, js_namespace = saveEditorFiles, js_name = pick)]
    fn js_pick() -> Result<js_sys::Promise, JsValue>;
    #[wasm_bindgen(catch, js_namespace = saveEditorFiles, js_name = write)]
    fn js_write(handle: &JsValue, bytes: &js_sys::Uint8Array, anyway: bool) -> Result<js_sys::Promise, JsValue>;
    #[wasm_bindgen(catch, js_namespace = saveEditorFiles, js_name = read)]
    fn js_read(handle: &JsValue) -> Result<js_sys::Promise, JsValue>;
    #[wasm_bindgen(catch, js_namespace = saveEditorFiles, js_name = accept)]
    fn js_accept(handle: &JsValue) -> Result<(), JsValue>;
    #[wasm_bindgen(catch, js_namespace = saveEditorFiles, js_name = originals)]
    fn js_originals() -> Result<js_sys::Promise, JsValue>;
    #[wasm_bindgen(catch, js_namespace = saveEditorFiles, js_name = downloadOriginal)]
    fn js_download_original(id: f64) -> Result<js_sys::Promise, JsValue>;
}

fn js_text(e: &JsValue) -> String {
    if let Some(s) = e.as_string() {
        return s;
    }
    if let Some(e) = e.dyn_ref::<js_sys::Error>() {
        return String::from(e.message());
    }
    format!("{e:?}")
}

fn field(v: &JsValue, name: &str) -> JsValue {
    js_sys::Reflect::get(v, &JsValue::from_str(name)).unwrap_or(JsValue::UNDEFINED)
}

fn field_text(v: &JsValue, name: &str) -> String {
    field(v, name).as_string().unwrap_or_default()
}

/// Starts the app in the `<canvas>` with this id.
#[wasm_bindgen]
pub async fn start(canvas_id: String) -> Result<(), JsValue> {
    let canvas = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(&canvas_id))
        .ok_or_else(|| JsValue::from_str("canvas not found"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;
    eframe::WebRunner::new()
        .start(canvas, eframe::WebOptions::default(), Box::new(|cc| Ok(Box::new(crate::app::App::new(cc, None)))))
        .await
}

/// Keeps a file the user gave the page, under its own name, and returns where it is. A handle
/// picked earlier under that name and not yet settled is let go.
pub fn keep(name: &str, data: &[u8]) -> PathBuf {
    let name = name.rsplit(['/', '\\']).next().filter(|n| !n.is_empty()).unwrap_or("save");
    let p = PathBuf::from(name);
    let _ = crate::vfs::write(&p, data);
    PENDING.with(|h| h.borrow_mut().remove(&p));
    p
}

/// Opens the browser's file picker; the chosen file arrives later through `take_picked`.
pub fn pick_file(ctx: &egui::Context) {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
    let Ok(input) = doc.create_element("input").and_then(|e| e.dyn_into::<web_sys::HtmlInputElement>().map_err(Into::into)) else {
        return;
    };
    input.set_type("file");
    let ctx = ctx.clone();
    let inp = input.clone();
    let on_change = Closure::once_into_js(move || {
        let Some(file) = inp.files().and_then(|f| f.get(0)) else { return };
        wasm_bindgen_futures::spawn_local(async move {
            let Ok(buf) = wasm_bindgen_futures::JsFuture::from(file.array_buffer()).await else { return };
            let data = js_sys::Uint8Array::new(&buf).to_vec();
            let p = keep(&file.name(), &data);
            PICKED.with(|q| q.borrow_mut().push(p));
            ctx.request_repaint();
        });
    });
    input.set_onchange(Some(on_change.unchecked_ref()));
    input.click();
}

/// Whether this browser can write back to a file the player picked (Chrome and Edge).
pub fn can_save_in_place() -> bool {
    thread_local! {
        static YES: Cell<Option<bool>> = const { Cell::new(None) };
    }
    YES.with(|y| {
        if y.get().is_none() {
            y.set(Some(js_supported().unwrap_or(false)));
        }
        y.get().unwrap_or(false)
    })
}

/// Opens the browser's own file picker, which also hands over a handle to write the file back
/// through. The file arrives through `take_picked` like any other, and `settle_handle` then
/// decides whether its handle is used. A failure arrives through `take_pick_error`.
pub fn pick_file_in_place(ctx: &egui::Context) {
    let promise = match js_pick() {
        Ok(p) => p,
        Err(e) => return PICK_ERROR.with(|f| *f.borrow_mut() = Some(js_text(&e))),
    };
    let ctx = ctx.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match wasm_bindgen_futures::JsFuture::from(promise).await {
            // The picker was closed.
            Ok(v) if v.is_null() || v.is_undefined() => {}
            Ok(v) => {
                let name = field_text(&v, "name");
                let data = js_sys::Uint8Array::new(&field(&v, "data")).to_vec();
                let p = keep(&name, &data);
                PENDING.with(|h| h.borrow_mut().insert(p.clone(), field(&v, "handle")));
                PICKED.with(|q| q.borrow_mut().push(p));
            }
            Err(e) => PICK_ERROR.with(|f| *f.borrow_mut() = Some(js_text(&e))),
        }
        ctx.request_repaint();
    });
}

/// What went wrong picking a file in place, once.
pub fn take_pick_error() -> Option<String> {
    PICK_ERROR.with(|f| f.borrow_mut().take())
}

/// The handle of the file just picked as `path`, if it was picked to save in place: with
/// `use_it`, its id for the document that opens it to hold; otherwise it is let go.
pub fn settle_handle(path: &Path, use_it: bool) -> Option<u64> {
    let h = PENDING.with(|h| h.borrow_mut().remove(path))?;
    if !use_it {
        return None;
    }
    let id = NEXT_ID.with(|n| n.replace(n.get() + 1));
    HANDLES.with(|m| m.borrow_mut().insert(id, h));
    Some(id)
}

/// Lets go of handle `id` (its document didn't open, or let go of it).
pub fn release_handle(id: u64) {
    HANDLES.with(|m| m.borrow_mut().remove(&id));
}

/// Makes the next save a download, whatever the file was opened with.
pub fn download_next() {
    DOWNLOAD_NEXT.with(|d| d.set(true));
}

/// Whether this save is to be a download; saves after it go where they normally do.
pub fn take_download_next() -> bool {
    DOWNLOAD_NEXT.with(|d| d.replace(false))
}

/// How writing back to the player's file went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InPlace {
    /// Written and read back as written. `kept`: the file as it was before this write is now
    /// kept in the browser (it differed from the copy kept last).
    Saved { kept: bool },
    /// The browser gave no permission to write (the player said no, or it couldn't ask). The
    /// file wasn't touched.
    Refused(String),
    /// The original couldn't be kept in the browser's storage, so nothing was written.
    NoBackup(String),
    /// Writing failed. `maybe_changed`: false when the file reads back as it was before, true
    /// when that can't be told.
    Failed { error: String, maybe_changed: bool },
    /// The file isn't what this page last read from or wrote to it (the game or another
    /// program wrote it meanwhile), so it wasn't touched.
    Changed,
    /// The handle is gone (nothing was written).
    NoHandle,
}

/// The answer of `saveEditorFiles.write`.
fn in_place_outcome(v: &JsValue) -> InPlace {
    let error = field_text(v, "error");
    match field_text(v, "status").as_str() {
        "saved" => InPlace::Saved { kept: field(v, "kept").as_bool().unwrap_or(false) },
        "refused" => InPlace::Refused(error),
        "nobackup" => InPlace::NoBackup(error),
        "changed" => InPlace::Changed,
        _ => InPlace::Failed { error, maybe_changed: field_text(v, "changed") != "no" },
    }
}

/// Writes `bytes` back to the file behind handle `id`, asking the browser for permission first,
/// then calls `done` with the outcome. Unless `anyway`, a file that changed since this page
/// last read or wrote it is left alone (`InPlace::Changed`). Whatever is in the file is kept
/// in the browser first when it differs from what was last kept there.
pub fn write_in_place(ctx: &egui::Context, id: u64, bytes: &[u8], anyway: bool, done: impl FnOnce(InPlace) + 'static) {
    let Some(handle) = HANDLES.with(|h| h.borrow().get(&id).cloned()) else {
        return done(InPlace::NoHandle);
    };
    // A copy the browser owns: a view into the program's memory could move while it writes.
    let promise = match js_write(&handle, &js_sys::Uint8Array::from(bytes), anyway) {
        Ok(p) => p,
        // Nothing was started, so nothing was written.
        Err(e) => return done(InPlace::Failed { error: js_text(&e), maybe_changed: false }),
    };
    let ctx = ctx.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let outcome = match wasm_bindgen_futures::JsFuture::from(promise).await {
            Ok(v) => in_place_outcome(&v),
            // `write` answers every failure itself; this is past anything it could check.
            Err(e) => InPlace::Failed { error: js_text(&e), maybe_changed: true },
        };
        done(outcome);
        ctx.request_repaint();
    });
}

/// Reads the file behind handle `id` again; `take_reread` has the bytes once they're in.
pub fn reread(ctx: &egui::Context, id: u64) {
    let set = move |r: Result<Vec<u8>, String>| REREAD.with(|x| *x.borrow_mut() = Some((id, r)));
    let Some(handle) = HANDLES.with(|h| h.borrow().get(&id).cloned()) else {
        return set(Err(String::new()));
    };
    let promise = match js_read(&handle) {
        Ok(p) => p,
        Err(e) => return set(Err(js_text(&e))),
    };
    let ctx = ctx.clone();
    wasm_bindgen_futures::spawn_local(async move {
        set(match wasm_bindgen_futures::JsFuture::from(promise).await {
            Ok(v) => Ok(js_sys::Uint8Array::new(&v).to_vec()),
            Err(e) => Err(js_text(&e)),
        });
        ctx.request_repaint();
    });
}

/// The file last read again through handle `id` is now on screen: from here on it is what the
/// page knows the file to hold, so a save over it no longer counts as writing over a change.
/// Until then, a save still finds the file changed and asks first.
pub fn accept_reread(id: u64) {
    if let Some(handle) = HANDLES.with(|h| h.borrow().get(&id).cloned()) {
        let _ = js_accept(&handle);
    }
}

/// The file read again by `reread`, once: the handle id and its bytes (or what went wrong).
pub fn take_reread() -> Option<(u64, Result<Vec<u8>, String>)> {
    REREAD.with(|x| x.borrow_mut().take())
}

/// A copy of a file as it was before it was first saved in place, kept in the browser.
#[derive(Debug, Clone)]
pub struct Original {
    pub id: f64,
    pub name: String,
    pub size: u64,
    /// When it was kept, written the browser's way.
    pub when: String,
}

/// Asks the browser for the kept originals; `originals` has them once it answers.
pub fn list_originals(ctx: &egui::Context) {
    ORIGINALS.with(|o| *o.borrow_mut() = None);
    let promise = match js_originals() {
        Ok(p) => p,
        Err(e) => return ORIGINALS.with(|o| *o.borrow_mut() = Some(Err(js_text(&e)))),
    };
    let ctx = ctx.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let r = match wasm_bindgen_futures::JsFuture::from(promise).await {
            Ok(v) => Ok(js_sys::Array::from(&v)
                .iter()
                .map(|o| Original {
                    id: field(&o, "id").as_f64().unwrap_or(-1.0),
                    name: field_text(&o, "name"),
                    size: field(&o, "size").as_f64().unwrap_or(0.0) as u64,
                    when: field_text(&o, "when"),
                })
                .collect()),
            Err(e) => Err(js_text(&e)),
        };
        ORIGINALS.with(|o| *o.borrow_mut() = Some(r));
        ctx.request_repaint();
    });
}

/// The kept originals: `None` while the browser is still listing them.
pub fn originals() -> Option<Result<Vec<Original>, String>> {
    ORIGINALS.with(|o| o.borrow().clone())
}

/// Hands kept original `id` to the browser as a download; `done` gets what went wrong, if
/// anything did (an empty text: it isn't kept any more).
pub fn download_original(ctx: &egui::Context, id: f64, done: impl FnOnce(Result<(), String>) + 'static) {
    let promise = match js_download_original(id) {
        Ok(p) => p,
        Err(e) => return done(Err(js_text(&e))),
    };
    let ctx = ctx.clone();
    wasm_bindgen_futures::spawn_local(async move {
        done(match wasm_bindgen_futures::JsFuture::from(promise).await {
            Ok(v) if v.as_bool() == Some(true) => Ok(()),
            Ok(_) => Err(String::new()),
            Err(e) => Err(js_text(&e)),
        });
        ctx.request_repaint();
    });
}

/// The next file the picker delivered, if any.
pub fn take_picked() -> Option<PathBuf> {
    PICKED.with(|q| {
        let mut q = q.borrow_mut();
        (!q.is_empty()).then(|| q.remove(0))
    })
}

/// Hands `data` to the browser as a download named `name`.
pub fn download(name: &str, data: &[u8]) -> Result<(), String> {
    let err = |e: JsValue| e.as_string().unwrap_or_else(|| format!("{e:?}"));
    let doc = web_sys::window().and_then(|w| w.document()).ok_or("no document")?;
    let parts = js_sys::Array::of1(&js_sys::Uint8Array::from(data));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type("application/octet-stream");
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &opts).map_err(err)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(err)?;
    let a = doc.create_element("a").map_err(err)?.dyn_into::<web_sys::HtmlAnchorElement>().map_err(|_| "no link")?;
    a.set_href(&url);
    a.set_download(name);
    a.click();
    // Let the browser start the download before the address is let go.
    let revoke = Closure::once_into_js(move || {
        let _ = web_sys::Url::revoke_object_url(&url);
    });
    if let Some(w) = web_sys::window() {
        let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(revoke.unchecked_ref(), 10_000);
    }
    Ok(())
}
